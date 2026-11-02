//! Minimal GeoTIFF window reader.
//!
//! Supports what the default predictors and typical user rasters need: classic TIFF and BigTIFF,
//! little/big endian, strips or tiles, single band, uncompressed / Deflate / LZW, horizontal
//! (2) and floating-point (3) predictors, integer and float sample types, GDAL NoData and
//! scale/offset metadata, EPSG:4326 georeferencing (PixelIsArea and PixelIsPoint), sparse chunks,
//! and — through [`GeoTiffInfo::read_allow_non_square`] — non-square pixels. Anything else is
//! rejected with an explicit error instead of being guessed (ADR-0003). Only the first IFD (full
//! resolution) is read; COG overviews are ignored.
//!
//! [`GeoTiffInfo::read_window`] returns a raster; [`GeoTiffInfo::visit_window`] streams decoded
//! chunks for aggregation of rasters too large to hold in memory (ESA WorldCover 10 m).
//!
//! Untrusted input: every size read from the file is bounds-checked against `MAX_*` limits before
//! allocation (decompression-bomb and huge-dimension protection).

use std::io::Read;

use crate::grid::{GridSpec, Raster};

const MAX_DIM: u64 = 1 << 20; // 1M pixels per side
const MAX_CHUNK_BYTES: u64 = 64 << 20;
const MAX_WINDOW_CELLS: u64 = 64 << 20;
/// Largest gap (bytes) between two chunks still fetched in the same range request.
const MAX_GAP: u64 = 4096;

#[derive(Debug, thiserror::Error)]
pub enum TiffError {
    #[error("I/O: {0}")]
    Io(String),
    #[error("not a TIFF file")]
    NotTiff,
    #[error("unsupported TIFF: {0}")]
    Unsupported(String),
    #[error("corrupt TIFF: {0}")]
    Corrupt(String),
    #[error(
        "raster is not in EPSG:4326 (found {0}); reproject it to WGS84 geographic coordinates first"
    )]
    Crs(String),
}

/// Random-access byte source (local file, HTTP range requests, memory).
pub trait ByteSource {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, TiffError>;
}

impl ByteSource for Vec<u8> {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, TiffError> {
        let s = usize::try_from(offset).map_err(|_| TiffError::Corrupt("offset".into()))?;
        self.get(s..s + len)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| TiffError::Corrupt("read past end".into()))
    }
}

/// Local file source (portable: seek + read under a mutex).
pub struct FileSource(std::sync::Mutex<std::fs::File>);

impl FileSource {
    pub fn open(path: &std::path::Path) -> Result<Self, TiffError> {
        std::fs::File::open(path)
            .map(|f| Self(std::sync::Mutex::new(f)))
            .map_err(|e| TiffError::Io(format!("{}: {e}", path.display())))
    }
}

impl ByteSource for FileSource {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, TiffError> {
        use std::io::{Seek, SeekFrom};
        let mut f = self
            .0
            .lock()
            .map_err(|_| TiffError::Io("file lock poisoned".into()))?;
        f.seek(SeekFrom::Start(offset))
            .map_err(|e| TiffError::Io(e.to_string()))?;
        let mut buf = vec![0u8; len];
        f.read_exact(&mut buf)
            .map_err(|e| TiffError::Io(e.to_string()))?;
        Ok(buf)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SampleType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    F64,
}

impl SampleType {
    fn bytes(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

/// One decoded internal tile (or strip) of a GeoTIFF, as passed to
/// [`GeoTiffInfo::visit_window`]. Pixel (r, c) of the raster, for `row0 ≤ r < row0 + height` and
/// `col0 ≤ c < col0 + width`, is `values[(r − row0) · stride + (c − col0)]`; NaN is NoData.
#[derive(Debug)]
pub struct Chunk<'a> {
    pub row0: usize,
    pub col0: usize,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub values: &'a [f32],
}

/// Parsed header of a single-band GeoTIFF.
#[derive(Debug, Clone)]
pub struct GeoTiffInfo {
    pub width: usize,
    pub height: usize,
    pub spec: GridSpec,
    pub nodata: Option<f64>,
    pub scale: f64,
    pub offset: f64,
    /// Pixel size in degrees along x (longitude) and y (latitude). Equal to `spec.cell` for
    /// square pixels; differs only for files opened with [`GeoTiffInfo::read_allow_non_square`]
    /// (e.g. Copernicus DEM tiles north of 50°, whose longitude spacing is coarser).
    pub pixel_x: f64,
    pub pixel_y: f64,
    le: bool,
    sample: SampleType,
    compression: u16,
    predictor: u16,
    chunk_w: usize,
    chunk_h: usize,
    tiled: bool,
    offsets: Vec<u64>,
    counts: Vec<u64>,
}

struct Cursor<'a> {
    b: &'a [u8],
    le: bool,
}

impl Cursor<'_> {
    fn u16(&self, o: usize) -> Result<u16, TiffError> {
        let s = self
            .b
            .get(o..o + 2)
            .ok_or_else(|| TiffError::Corrupt("short read".into()))?;
        let a = [s[0], s[1]];
        Ok(if self.le {
            u16::from_le_bytes(a)
        } else {
            u16::from_be_bytes(a)
        })
    }
    fn u32(&self, o: usize) -> Result<u32, TiffError> {
        let s = self
            .b
            .get(o..o + 4)
            .ok_or_else(|| TiffError::Corrupt("short read".into()))?;
        let a = [s[0], s[1], s[2], s[3]];
        Ok(if self.le {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    }
    fn u64(&self, o: usize) -> Result<u64, TiffError> {
        let s = self
            .b
            .get(o..o + 8)
            .ok_or_else(|| TiffError::Corrupt("short read".into()))?;
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        Ok(if self.le {
            u64::from_le_bytes(a)
        } else {
            u64::from_be_bytes(a)
        })
    }
}

#[derive(Debug, Clone)]
struct Entry {
    tag: u16,
    typ: u16,
    count: u64,
    /// Raw value bytes (inline or fetched).
    data: Vec<u8>,
}

fn type_size(t: u16) -> Option<usize> {
    Some(match t {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => return None,
    })
}

impl Entry {
    fn values_u64(&self, le: bool) -> Result<Vec<u64>, TiffError> {
        let c = Cursor { b: &self.data, le };
        let n = usize::try_from(self.count).map_err(|_| TiffError::Corrupt("count".into()))?;
        (0..n)
            .map(|i| match self.typ {
                1 | 7 => Ok(u64::from(self.data[i])),
                3 => c.u16(i * 2).map(u64::from),
                4 | 13 => c.u32(i * 4).map(u64::from),
                16 | 18 => c.u64(i * 8),
                t => Err(TiffError::Unsupported(format!("integer tag type {t}"))),
            })
            .collect()
    }
    fn values_f64(&self, le: bool) -> Result<Vec<f64>, TiffError> {
        let c = Cursor { b: &self.data, le };
        let n = usize::try_from(self.count).map_err(|_| TiffError::Corrupt("count".into()))?;
        (0..n)
            .map(|i| match self.typ {
                12 => c.u64(i * 8).map(f64::from_bits),
                11 => c.u32(i * 4).map(|b| f64::from(f32::from_bits(b))),
                _ => self.values_u64(le).map(|v| v[i] as f64),
            })
            .collect()
    }
    fn ascii(&self) -> String {
        String::from_utf8_lossy(&self.data)
            .trim_end_matches('\0')
            .to_string()
    }
}

impl GeoTiffInfo {
    /// Parse the first IFD (full-resolution image). Non-square pixels are rejected.
    pub fn read(src: &dyn ByteSource) -> Result<Self, TiffError> {
        Self::read_impl(src, false)
    }

    /// Like [`GeoTiffInfo::read`] but accepting pixels whose x and y sizes differ. `spec.cell` is
    /// then `pixel_y`, and only [`GeoTiffInfo::visit_window`] may be used (it reports pixel
    /// indices; georeferencing is the caller's job with `pixel_x`/`pixel_y`).
    pub fn read_allow_non_square(src: &dyn ByteSource) -> Result<Self, TiffError> {
        Self::read_impl(src, true)
    }

    fn read_impl(src: &dyn ByteSource, allow_non_square: bool) -> Result<Self, TiffError> {
        let head = src.read_at(0, 16)?;
        let le = match &head[0..2] {
            b"II" => true,
            b"MM" => false,
            _ => return Err(TiffError::NotTiff),
        };
        let c = Cursor { b: &head, le };
        let magic = c.u16(2)?;
        let big = match magic {
            42 => false,
            43 => true,
            _ => return Err(TiffError::NotTiff),
        };
        let ifd_off = if big { c.u64(8)? } else { u64::from(c.u32(4)?) };
        let (cnt_size, entry_size) = if big { (8, 20) } else { (2, 12) };
        let cnt_buf = src.read_at(ifd_off, cnt_size)?;
        let cc = Cursor { b: &cnt_buf, le };
        let n = if big {
            cc.u64(0)?
        } else {
            u64::from(cc.u16(0)?)
        };
        if n > 4096 {
            return Err(TiffError::Corrupt("too many IFD entries".into()));
        }
        let ents = src.read_at(ifd_off + cnt_size as u64, n as usize * entry_size)?;
        let ec = Cursor { b: &ents, le };
        let mut entries = Vec::new();
        for i in 0..n as usize {
            let o = i * entry_size;
            let tag = ec.u16(o)?;
            let typ = ec.u16(o + 2)?;
            let count = if big {
                ec.u64(o + 4)?
            } else {
                u64::from(ec.u32(o + 4)?)
            };
            let Some(ts) = type_size(typ) else { continue };
            let total = count
                .checked_mul(ts as u64)
                .ok_or_else(|| TiffError::Corrupt("tag size".into()))?;
            if total > MAX_CHUNK_BYTES {
                return Err(TiffError::Corrupt("tag too large".into()));
            }
            let inline = if big { 8 } else { 4 };
            let voff = o + if big { 12 } else { 8 };
            let data = if total as usize <= inline {
                ents[voff..voff + total as usize].to_vec()
            } else {
                let off = if big {
                    ec.u64(voff)?
                } else {
                    u64::from(ec.u32(voff)?)
                };
                src.read_at(off, total as usize)?
            };
            entries.push(Entry {
                tag,
                typ,
                count,
                data,
            });
        }
        let get = |t: u16| entries.iter().find(|e| e.tag == t);
        let one = |t: u16| -> Result<Option<u64>, TiffError> {
            get(t).map(|e| e.values_u64(le).map(|v| v[0])).transpose()
        };

        let width = one(256)?.ok_or_else(|| TiffError::Corrupt("no width".into()))?;
        let height = one(257)?.ok_or_else(|| TiffError::Corrupt("no height".into()))?;
        if width == 0 || height == 0 || width > MAX_DIM || height > MAX_DIM {
            return Err(TiffError::Unsupported(format!(
                "dimensions {width}x{height}"
            )));
        }
        let spp = one(277)?.unwrap_or(1);
        if spp != 1 {
            return Err(TiffError::Unsupported(format!(
                "{spp} samples per pixel (single band only)"
            )));
        }
        let bits = one(258)?.unwrap_or(1);
        let fmt = one(339)?.unwrap_or(1);
        let sample = match (fmt, bits) {
            (1, 8) => SampleType::U8,
            (2, 8) => SampleType::I8,
            (1, 16) => SampleType::U16,
            (2, 16) => SampleType::I16,
            (1, 32) => SampleType::U32,
            (2, 32) => SampleType::I32,
            (3, 32) => SampleType::F32,
            (3, 64) => SampleType::F64,
            _ => {
                return Err(TiffError::Unsupported(format!(
                    "sample format {fmt} with {bits} bits"
                )));
            }
        };
        let compression = one(259)?.unwrap_or(1) as u16;
        if !matches!(compression, 1 | 5 | 8 | 32946) {
            return Err(TiffError::Unsupported(format!(
                "compression {compression} (supported: none, LZW, Deflate)"
            )));
        }
        let predictor = one(317)?.unwrap_or(1) as u16;
        if !matches!(predictor, 1..=3) {
            return Err(TiffError::Unsupported(format!("predictor {predictor}")));
        }
        let planar = one(284)?.unwrap_or(1);
        if planar != 1 {
            return Err(TiffError::Unsupported("planar configuration".into()));
        }
        let (tiled, chunk_w, chunk_h, offsets, counts) = if let Some(tw) = one(322)? {
            let th = one(323)?.ok_or_else(|| TiffError::Corrupt("tile length".into()))?;
            let o = get(324)
                .ok_or_else(|| TiffError::Corrupt("tile offsets".into()))?
                .values_u64(le)?;
            let cnts = get(325)
                .ok_or_else(|| TiffError::Corrupt("tile byte counts".into()))?
                .values_u64(le)?;
            (true, tw as usize, th as usize, o, cnts)
        } else {
            let rps = one(278)?.unwrap_or(height).min(height);
            let o = get(273)
                .ok_or_else(|| TiffError::Corrupt("strip offsets".into()))?
                .values_u64(le)?;
            let cnts = get(279)
                .ok_or_else(|| TiffError::Corrupt("strip byte counts".into()))?
                .values_u64(le)?;
            (false, width as usize, rps as usize, o, cnts)
        };
        if chunk_w == 0 || chunk_h == 0 || offsets.len() != counts.len() {
            return Err(TiffError::Corrupt("chunk layout".into()));
        }
        let (width, height) = (width as usize, height as usize);
        let expected = width.div_ceil(chunk_w) * height.div_ceil(chunk_h);
        if offsets.len() < expected {
            return Err(TiffError::Corrupt("missing chunks".into()));
        }

        // Georeferencing: ModelPixelScale (33550) + ModelTiepoint (33922), GeoKeys (34735).
        let scale = get(33550)
            .ok_or_else(|| TiffError::Crs("no georeferencing".into()))?
            .values_f64(le)?;
        let tie = get(33922)
            .ok_or_else(|| TiffError::Crs("no georeferencing".into()))?
            .values_f64(le)?;
        if scale.len() < 2 || tie.len() < 6 {
            return Err(TiffError::Corrupt("georeferencing tags".into()));
        }
        if !allow_non_square && (scale[0] - scale[1]).abs() > 1e-9 * scale[0].abs() {
            return Err(TiffError::Unsupported("non-square cells".into()));
        }
        if !(scale[0] > 0.0 && scale[1] > 0.0) {
            return Err(TiffError::Unsupported("non-positive pixel size".into()));
        }
        let keys = get(34735)
            .map(|e| e.values_u64(le))
            .transpose()?
            .unwrap_or_default();
        let mut epsg = None;
        let mut raster_is_point = false;
        for k in keys.chunks(4).skip(1) {
            if k.len() < 4 {
                break;
            }
            match k[0] {
                2048 if k[1] == 0 => epsg = Some(k[3]),
                1025 if k[1] == 0 => raster_is_point = k[3] == 2,
                1024 if k[1] == 0 && k[3] != 2 => {
                    return Err(TiffError::Crs(format!("model type {}", k[3])));
                }
                _ => {}
            }
        }
        match epsg {
            Some(4326) => {}
            Some(other) => return Err(TiffError::Crs(format!("geographic CRS EPSG:{other}"))),
            None => return Err(TiffError::Crs("unspecified geographic CRS".into())),
        }
        let (pixel_x, pixel_y) = (scale[0], scale[1]);
        let cell = pixel_y;
        // PixelIsArea: the tie point is the outer corner of pixel (i, j). PixelIsPoint: it is the
        // pixel centre, so the outer corner is half a pixel up and left (GDAL's convention).
        let (mut west, mut north) = (tie[3] - tie[0] * pixel_x, tie[4] + tie[1] * pixel_y);
        if raster_is_point {
            west -= pixel_x / 2.0;
            north += pixel_y / 2.0;
        }
        let nodata = get(42113).and_then(|e| e.ascii().trim().parse::<f64>().ok());
        let (mut s, mut o) = (1.0, 0.0);
        if let Some(md) = get(42112) {
            let xml = md.ascii();
            if let Some(v) = gdal_item(&xml, "scale") {
                s = v;
            }
            if let Some(v) = gdal_item(&xml, "offset") {
                o = v;
            }
        }
        Ok(Self {
            width,
            height,
            spec: GridSpec {
                west,
                north,
                cell,
                width,
                height,
            },
            nodata,
            scale: s,
            offset: o,
            pixel_x,
            pixel_y,
            le,
            sample,
            compression,
            predictor,
            chunk_w,
            chunk_h,
            tiled,
            offsets,
            counts,
        })
    }

    /// Read the pixel window [row0, row0+rows) × [col0, col0+cols) as a raster (scale/offset
    /// applied, NoData → NaN).
    pub fn read_window(
        &self,
        src: &dyn ByteSource,
        row0: usize,
        col0: usize,
        rows: usize,
        cols: usize,
    ) -> Result<Raster, TiffError> {
        if (self.pixel_x - self.pixel_y).abs() > 1e-9 * self.pixel_y.abs() {
            return Err(TiffError::Unsupported(
                "non-square cells: use visit_window".into(),
            ));
        }
        if (rows as u64) * (cols as u64) > MAX_WINDOW_CELLS {
            return Err(TiffError::Unsupported("window too large".into()));
        }
        let spec = GridSpec {
            west: self.spec.west + col0 as f64 * self.spec.cell,
            north: self.spec.north - row0 as f64 * self.spec.cell,
            cell: self.spec.cell,
            width: cols,
            height: rows,
        };
        let mut out = Raster::new(spec, f32::NAN);
        self.visit_window(src, row0, col0, rows, cols, &mut |ch: &Chunk| {
            let r_lo = ch.row0.max(row0);
            let r_hi = (ch.row0 + ch.height).min(row0 + rows);
            let c_lo = ch.col0.max(col0);
            let c_hi = (ch.col0 + ch.width).min(col0 + cols);
            for gr in r_lo..r_hi {
                let src_row = &ch.values[(gr - ch.row0) * ch.stride..];
                let dst = spec.index(gr - row0, 0);
                for gc in c_lo..c_hi {
                    out.data[dst + gc - col0] = src_row[gc - ch.col0];
                }
            }
            Ok(())
        })?;
        Ok(out)
    }

    /// Decode, one internal tile or strip at a time, every chunk intersecting the pixel window
    /// [row0, row0+rows) × [col0, col0+cols), and pass it to `f` (scale/offset applied,
    /// NoData → NaN). Chunks are delivered whole (clipped to the raster, not to the window), so
    /// the caller filters pixels. Memory stays bounded by one batch of compressed bytes (≤ 8 MiB
    /// of contiguous chunks fetched per request) plus one decoded chunk: suitable for streaming
    /// aggregation of very large rasters.
    pub fn visit_window(
        &self,
        src: &dyn ByteSource,
        row0: usize,
        col0: usize,
        rows: usize,
        cols: usize,
        f: &mut dyn FnMut(&Chunk) -> Result<(), TiffError>,
    ) -> Result<(), TiffError> {
        if row0 + rows > self.height || col0 + cols > self.width || rows == 0 || cols == 0 {
            return Err(TiffError::Corrupt("window outside raster".into()));
        }
        let across = self.width.div_ceil(self.chunk_w);
        let (cr0, cr1) = (row0 / self.chunk_h, (row0 + rows - 1) / self.chunk_h);
        let (cc0, cc1) = if self.tiled {
            (col0 / self.chunk_w, (col0 + cols - 1) / self.chunk_w)
        } else {
            (0, 0)
        };
        // Collect needed chunks, then fetch (nearly) contiguous byte ranges in batches: few HTTP
        // requests. GDAL-written COGs put a 4-byte size leader and trailer around each tile, so
        // consecutive tiles are separated by a few bytes; gaps up to `MAX_GAP` are read and
        // skipped.
        let mut needed: Vec<(usize, usize, usize)> = Vec::new(); // (chunk index, chunk row, chunk col)
        for cr in cr0..=cr1 {
            for cc in cc0..=cc1 {
                needed.push((cr * across + cc, cr, cc));
            }
        }
        needed.sort_by_key(|&(i, _, _)| self.offsets[i]);
        let mut batch_start = 0;
        while batch_start < needed.len() {
            let mut end = batch_start + 1;
            let first = needed[batch_start].0;
            let mut stop = self.offsets[first] + self.counts[first];
            while end < needed.len() {
                let i = needed[end].0;
                if self.offsets[i] < stop
                    || self.offsets[i] - stop > MAX_GAP
                    || stop - self.offsets[first] > 8 << 20
                {
                    break;
                }
                stop = self.offsets[i] + self.counts[i];
                end += 1;
            }
            let len = stop - self.offsets[first];
            if len > MAX_CHUNK_BYTES * 2 {
                return Err(TiffError::Corrupt("chunk too large".into()));
            }
            let bytes = src.read_at(self.offsets[first], len as usize)?;
            for &(i, cr, cc) in &needed[batch_start..end] {
                let n = self.counts[i] as usize;
                let pos = (self.offsets[i] - self.offsets[first]) as usize;
                let raw = bytes
                    .get(pos..pos + n)
                    .ok_or_else(|| TiffError::Corrupt("chunk bounds".into()))?;
                // Sparse files (GDAL SPARSE_OK) omit empty chunks: offset/count 0 = all NoData.
                let (values, stride, ch) = if n == 0 {
                    let ch = if self.tiled {
                        self.chunk_h
                    } else {
                        self.chunk_h.min(self.height - cr * self.chunk_h)
                    };
                    (vec![f32::NAN; self.chunk_w * ch], self.chunk_w, ch)
                } else {
                    self.decode_chunk(raw, cr)?
                };
                let chunk = Chunk {
                    row0: cr * self.chunk_h,
                    col0: cc * self.chunk_w,
                    width: stride.min(self.width - cc * self.chunk_w),
                    height: ch.min(self.height - cr * self.chunk_h),
                    stride,
                    values: &values,
                };
                f(&chunk)?;
            }
            batch_start = end;
        }
        Ok(())
    }

    /// Decompress and convert one chunk to f32 values (row-major, `stride` = chunk width).
    /// Returns (values, stride, chunk height).
    fn decode_chunk(&self, raw: &[u8], cr: usize) -> Result<(Vec<f32>, usize, usize), TiffError> {
        let bps = self.sample.bytes();
        // Strips: the last strip may be shorter.
        let ch = if self.tiled {
            self.chunk_h
        } else {
            self.chunk_h.min(self.height - cr * self.chunk_h)
        };
        let cw = self.chunk_w;
        let expected = cw * ch * bps;
        if expected as u64 > MAX_CHUNK_BYTES {
            return Err(TiffError::Unsupported("chunk too large".into()));
        }
        let mut buf = match self.compression {
            1 => raw.to_vec(),
            8 | 32946 => {
                let mut d = flate2::read::ZlibDecoder::new(raw).take(expected as u64 + 1);
                let mut v = Vec::with_capacity(expected);
                d.read_to_end(&mut v)
                    .map_err(|e| TiffError::Corrupt(format!("deflate: {e}")))?;
                v
            }
            5 => {
                let mut dec =
                    weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
                let mut v = Vec::with_capacity(expected);
                let r = dec.into_vec(&mut v).decode(raw);
                r.status
                    .map_err(|e| TiffError::Corrupt(format!("lzw: {e}")))?;
                v
            }
            c => return Err(TiffError::Unsupported(format!("compression {c}"))),
        };
        if buf.len() < expected {
            return Err(TiffError::Corrupt(format!(
                "chunk decoded to {} bytes, expected {expected}",
                buf.len()
            )));
        }
        buf.truncate(expected);
        self.unpredict(&mut buf, cw, ch);
        let mut values = Vec::with_capacity(cw * ch);
        for k in 0..cw * ch {
            let v = self.sample_at(&buf, k * bps);
            values.push(if self.nodata.is_some_and(|nd| v == nd) || v.is_nan() {
                f32::NAN
            } else {
                (v * self.scale + self.offset) as f32
            });
        }
        Ok((values, cw, ch))
    }

    fn sample_at(&self, b: &[u8], o: usize) -> f64 {
        let le = self.le;
        macro_rules! rd {
            ($t:ty, $n:expr) => {{
                let mut a = [0u8; $n];
                a.copy_from_slice(&b[o..o + $n]);
                if le {
                    <$t>::from_le_bytes(a)
                } else {
                    <$t>::from_be_bytes(a)
                }
            }};
        }
        match self.sample {
            SampleType::U8 => f64::from(b[o]),
            SampleType::I8 => f64::from(b[o] as i8),
            SampleType::U16 => f64::from(rd!(u16, 2)),
            SampleType::I16 => f64::from(rd!(i16, 2)),
            SampleType::U32 => f64::from(rd!(u32, 4)),
            SampleType::I32 => f64::from(rd!(i32, 4)),
            SampleType::F32 => f64::from(rd!(f32, 4)),
            SampleType::F64 => rd!(f64, 8),
        }
    }

    fn unpredict(&self, buf: &mut [u8], w: usize, h: usize) {
        let bps = self.sample.bytes();
        match self.predictor {
            2 => {
                for r in 0..h {
                    let row = &mut buf[r * w * bps..(r + 1) * w * bps];
                    match bps {
                        1 => {
                            for c in 1..w {
                                row[c] = row[c].wrapping_add(row[c - 1]);
                            }
                        }
                        2 => {
                            for c in 1..w {
                                let get = |s: &[u8], i: usize| {
                                    if self.le {
                                        u16::from_le_bytes([s[i * 2], s[i * 2 + 1]])
                                    } else {
                                        u16::from_be_bytes([s[i * 2], s[i * 2 + 1]])
                                    }
                                };
                                let v = get(row, c).wrapping_add(get(row, c - 1));
                                let b = if self.le {
                                    v.to_le_bytes()
                                } else {
                                    v.to_be_bytes()
                                };
                                row[c * 2..c * 2 + 2].copy_from_slice(&b);
                            }
                        }
                        4 => {
                            for c in 1..w {
                                let get = |s: &[u8], i: usize| {
                                    let a = [s[i * 4], s[i * 4 + 1], s[i * 4 + 2], s[i * 4 + 3]];
                                    if self.le {
                                        u32::from_le_bytes(a)
                                    } else {
                                        u32::from_be_bytes(a)
                                    }
                                };
                                let v = get(row, c).wrapping_add(get(row, c - 1));
                                let b = if self.le {
                                    v.to_le_bytes()
                                } else {
                                    v.to_be_bytes()
                                };
                                row[c * 4..c * 4 + 4].copy_from_slice(&b);
                            }
                        }
                        _ => {}
                    }
                }
            }
            3 => {
                // Floating-point predictor: byte-wise differencing over byte-planes, big-endian order.
                for r in 0..h {
                    let row = &mut buf[r * w * bps..(r + 1) * w * bps];
                    for i in 1..row.len() {
                        row[i] = row[i].wrapping_add(row[i - 1]);
                    }
                    let tmp = row.to_vec();
                    for c in 0..w {
                        for k in 0..bps {
                            let src = tmp[k * w + c];
                            let dst = if self.le {
                                c * bps + (bps - 1 - k)
                            } else {
                                c * bps + k
                            };
                            row[dst] = src;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pixel window covering a lon/lat bounding box, clipped to the raster.
    pub fn window_for_bbox(
        &self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
    ) -> Option<(usize, usize, usize, usize)> {
        let s = &self.spec;
        let c0 = ((west - s.west) / s.cell).floor().max(0.0) as usize;
        let c1 = (((east - s.west) / s.cell).ceil() as usize).min(self.width);
        let r0 = ((s.north - north) / s.cell).floor().max(0.0) as usize;
        let r1 = (((s.north - south) / s.cell).ceil() as usize).min(self.height);
        if c1 <= c0 || r1 <= r0 {
            return None;
        }
        Some((r0, c0, r1 - r0, c1 - c0))
    }
}

/// Value of the `<Item ... role="scale|offset">value</Item>` in GDAL_METADATA XML. GDAL writes
/// `name="SCALE" role="scale"`; matching on the role is case-stable.
fn gdal_item(xml: &str, name: &str) -> Option<f64> {
    let needle = format!("role=\"{name}\"");
    let start = xml.find(&needle)?;
    let rest = &xml[start..];
    let gt = rest.find('>')?;
    let end = rest.find("</Item>")?;
    rest.get(gt + 1..end)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::write_geotiff_f32;

    #[test]
    fn roundtrip_with_own_writer() {
        let spec = GridSpec {
            west: -5.0,
            north: 51.0,
            cell: 0.25,
            width: 300,
            height: 270,
        };
        let mut r = Raster::new(spec, 0.0);
        for (i, v) in r.data.iter_mut().enumerate() {
            *v = if i % 17 == 0 {
                f32::NAN
            } else {
                (i as f32).sin() * 100.0
            };
        }
        let bytes = write_geotiff_f32(&r).expect("write");
        let info = GeoTiffInfo::read(&bytes).expect("read");
        assert_eq!(info.spec, spec);
        let w = info.read_window(&bytes, 10, 20, 100, 200).expect("window");
        for rr in 0..100 {
            for cc in 0..200 {
                let a = r.get(rr + 10, cc + 20);
                let b = w.get(rr, cc);
                assert!(a == b || (a.is_nan() && b.is_nan()));
            }
        }
    }

    #[test]
    fn gdal_metadata_parsing() {
        let xml = "<GDALMetadata>\n  <Item name=\"OFFSET\" sample=\"0\" role=\"offset\">-273.149999999999977</Item>\n  <Item name=\"SCALE\" sample=\"0\" role=\"scale\">0.100000000000000006</Item>\n</GDALMetadata>";
        assert_eq!(gdal_item(xml, "scale"), Some(0.1));
        assert_eq!(gdal_item(xml, "offset"), Some(-273.15));
    }

    #[test]
    fn rejects_garbage() {
        assert!(GeoTiffInfo::read(&b"hello world, not a tiff".to_vec()).is_err());
    }

    fn small_tiff(west: f64, north: f64, cell: f64) -> (Raster, Vec<u8>) {
        let spec = GridSpec {
            west,
            north,
            cell,
            width: 300,
            height: 260,
        };
        let mut r = Raster::new(spec, 0.0);
        for (i, v) in r.data.iter_mut().enumerate() {
            *v = (i % 1000) as f32 * 0.5 - 7.25;
        }
        let bytes = write_geotiff_f32(&r).expect("write");
        (r, bytes)
    }

    fn patch(bytes: &mut [u8], from: &[u8], to: &[u8]) {
        let pos = bytes
            .windows(from.len())
            .position(|w| w == from)
            .expect("pattern present");
        bytes[pos..pos + to.len()].copy_from_slice(to);
    }

    /// TIFF Technical Note 3 floating-point predictor (used by the Copernicus DEM COGs): encode a
    /// row by splitting the big-endian bytes into planes and differencing bytes, then check the
    /// decoder restores the floats exactly.
    #[test]
    fn floating_point_predictor_decodes() {
        let (_, bytes) = small_tiff(0.0, 1.0, 0.01);
        let mut info = GeoTiffInfo::read(&bytes).expect("read");
        info.predictor = 3;
        let w = 7;
        let vals: [f32; 7] = [1.5, -2.25, 1234.567, 0.0, f32::MAX, -0.001, 8848.86];
        let mut planes = vec![0u8; w * 4];
        for (c, v) in vals.iter().enumerate() {
            for (k, b) in v.to_be_bytes().iter().enumerate() {
                planes[k * w + c] = *b;
            }
        }
        let mut enc = planes.clone();
        for i in (1..enc.len()).rev() {
            enc[i] = planes[i].wrapping_sub(planes[i - 1]);
        }
        let mut row = enc.clone();
        row.extend_from_slice(&enc);
        info.unpredict(&mut row, w, 2);
        for r in 0..2 {
            for (c, v) in vals.iter().enumerate() {
                let o = (r * w + c) * 4;
                let got = f32::from_le_bytes([row[o], row[o + 1], row[o + 2], row[o + 3]]);
                assert_eq!(got.to_bits(), v.to_bits());
            }
        }
    }

    /// GeoKey RasterPixelIsPoint (1025 = 2): the tie point is a pixel centre, so the grid edge is
    /// half a pixel up-left (Copernicus DEM convention).
    #[test]
    fn pixel_is_point_shifts_half_a_pixel() {
        let (_, mut bytes) = small_tiff(5.0, 44.0, 0.01);
        let key = |v: u16| -> Vec<u8> {
            [1025u16, 0, 1, v]
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect()
        };
        patch(&mut bytes, &key(1), &key(2));
        let info = GeoTiffInfo::read(&bytes).expect("read");
        assert!((info.spec.west - (5.0 - 0.005)).abs() < 1e-12);
        assert!((info.spec.north - (44.0 + 0.005)).abs() < 1e-12);
    }

    /// Non-square pixels (Copernicus DEM above 50° N) are refused by `read` and accepted by
    /// `read_allow_non_square`; `visit_window` delivers the same values as a square read.
    #[test]
    fn non_square_pixels_and_visitor() {
        let (r, mut bytes) = small_tiff(2.0, 51.0, 0.01);
        patch(&mut bytes, &0.01f64.to_le_bytes(), &0.015f64.to_le_bytes());
        assert!(GeoTiffInfo::read(&bytes).is_err());
        let info = GeoTiffInfo::read_allow_non_square(&bytes).expect("read");
        assert!((info.pixel_x - 0.015).abs() < 1e-15 && (info.pixel_y - 0.01).abs() < 1e-15);
        assert!(info.read_window(&bytes, 0, 0, 5, 5).is_err());
        let mut seen = 0usize;
        info.visit_window(&bytes, 250, 10, 10, 280, &mut |ch: &Chunk| {
            for rr in 0..ch.height {
                for cc in 0..ch.width {
                    let v = ch.values[rr * ch.stride + cc];
                    assert_eq!(v, r.get(ch.row0 + rr, ch.col0 + cc));
                    seen += 1;
                }
            }
            Ok(())
        })
        .expect("visit");
        // 256×256 tiles: rows 250..260 touch both tile rows and cols 10..290 both tile columns,
        // so the whole 260×300 raster is delivered (chunks are not clipped to the window).
        assert_eq!(seen, 260 * 300);
    }
}
