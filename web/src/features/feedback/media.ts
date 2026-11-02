// Browser media for the feedback form: one-frame screen capture, voice recording, image
// downscaling and base64 for the JSON upload. No dependency.

import { useEffect, useRef, useState } from "react";

/** Same limits as the server (crates/niche-app/src/feedback.rs). */
export const MAX_FILES = 6;
export const MAX_BYTES = 12 << 20;
const MAX_RECORDING_S = 5 * 60;
/** Images above this are re-encoded smaller before sending. */
const SHRINK_ABOVE = 2 << 20;
const MAX_SIDE = 2560;

export interface Attachment {
  id: string;
  name: string;
  mime: string;
  blob: Blob;
  url: string;
}

let seq = 0;
export function attachment(blob: Blob, name: string): Attachment {
  seq += 1;
  return { id: `a${seq}`, name, mime: blob.type, blob, url: URL.createObjectURL(blob) };
}

const stamp = () => new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");

function canvasBlob(canvas: HTMLCanvasElement): Promise<Blob | null> {
  // WebP where supported (much smaller than PNG for screenshots); PNG otherwise.
  return new Promise((resolve) => canvas.toBlob((b) => resolve(b), "image/webp", 0.85));
}

/** One frame of a screen, window or tab the user picks (the browser asks first). */
export async function captureScreen(): Promise<Attachment> {
  const stream = await navigator.mediaDevices.getDisplayMedia({
    video: { displaySurface: "browser" },
    audio: false,
    // Chromium: offer this tab first.
    preferCurrentTab: true,
  } as DisplayMediaStreamOptions);
  try {
    const video = document.createElement("video");
    video.srcObject = stream;
    video.muted = true;
    await video.play();
    // Let the picker disappear from the captured surface.
    await new Promise((r) => setTimeout(r, 350));
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    canvas.getContext("2d")?.drawImage(video, 0, 0);
    video.pause();
    const blob = await canvasBlob(canvas);
    if (!blob) throw new Error("capture failed");
    return attachment(blob, `screen-${stamp()}.${blob.type === "image/webp" ? "webp" : "png"}`);
  } finally {
    stream.getTracks().forEach((t) => t.stop());
  }
}

/** Large photos and screenshots are scaled down to MAX_SIDE and re-encoded; small ones kept. */
export async function shrinkImage(file: File): Promise<Attachment> {
  if (file.size <= SHRINK_ABOVE || file.type === "image/gif") return attachment(file, file.name || `image-${stamp()}.png`);
  try {
    const bitmap = await createImageBitmap(file);
    const scale = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height));
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(bitmap.width * scale);
    canvas.height = Math.round(bitmap.height * scale);
    canvas.getContext("2d")?.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    bitmap.close();
    const blob = await canvasBlob(canvas);
    if (!blob || blob.size >= file.size) return attachment(file, file.name);
    const base = (file.name || "image").replace(/\.[^.]+$/, "");
    return attachment(blob, `${base}.${blob.type === "image/webp" ? "webp" : "png"}`);
  } catch {
    return attachment(file, file.name);
  }
}

export function toBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(typeof r.result === "string" ? r.result.replace(/^data:[^,]*,/, "") : "");
    r.onerror = () => reject(r.error ?? new Error("read failed"));
    r.readAsDataURL(blob);
  });
}

const AUDIO_TYPES = ["audio/webm;codecs=opus", "audio/webm", "audio/ogg;codecs=opus", "audio/mp4"];

// Feature checks only: the methods are called on `navigator.mediaDevices` itself.
export const canRecord = () => typeof MediaRecorder !== "undefined" && "getUserMedia" in (navigator.mediaDevices ?? {});
export const canCapture = () => "getDisplayMedia" in (navigator.mediaDevices ?? {});

/** Voice recorder: start() asks for the microphone; stop() (or 5 minutes) yields the recording. */
export function useRecorder(onDone: (a: Attachment) => void, onError: () => void) {
  const [seconds, setSeconds] = useState<number | null>(null);
  const rec = useRef<MediaRecorder | null>(null);
  const done = useRef(onDone);
  const failed = useRef(onError);
  useEffect(() => {
    done.current = onDone;
    failed.current = onError;
  });
  useEffect(() => {
    if (seconds === null) return;
    if (seconds >= MAX_RECORDING_S) {
      rec.current?.stop();
      return;
    }
    const t = setTimeout(() => setSeconds((s) => (s === null ? null : s + 1)), 1000);
    return () => clearTimeout(t);
  }, [seconds]);
  // Closing the form mid-recording releases the microphone.
  useEffect(() => () => rec.current?.stream.getTracks().forEach((t) => t.stop()), []);

  const start = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mimeType = AUDIO_TYPES.find((t) => MediaRecorder.isTypeSupported(t));
      const r = new MediaRecorder(stream, mimeType ? { mimeType } : undefined);
      const chunks: Blob[] = [];
      r.ondataavailable = (e) => e.data.size > 0 && chunks.push(e.data);
      r.onstop = () => {
        stream.getTracks().forEach((t) => t.stop());
        rec.current = null;
        setSeconds(null);
        const type = (r.mimeType || mimeType || "audio/webm").split(";")[0] ?? "audio/webm";
        const blob = new Blob(chunks, { type });
        if (blob.size > 0) done.current(attachment(blob, `voice-${stamp()}.${type === "audio/mp4" ? "m4a" : type.split("/")[1]}`));
      };
      rec.current = r;
      r.start(1000);
      setSeconds(0);
    } catch {
      failed.current();
    }
  };
  const stop = () => rec.current?.stop();
  return { seconds, start, stop };
}

export const clock = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
