import { X } from "lucide-react";
import { Fragment } from "react";

import { errorMessage } from "@/api/client";
import { useSample } from "@/api/hooks";
import { Skeleton } from "@/components/ui/skeleton";
import { useProjectionMessages } from "@/features/projections/messages";
import { formatCount, formatDecimal, formatLngLat, formatScore } from "@/lib/format";
import { useMapStore } from "@/stores/map";

import { useMapMessages } from "./messages";

/** Values at a clicked location (GET …/sample). Rendered inside the MapLibre popup. */
export function InspectContent() {
  const msg = useMapMessages();
  const at = useMapStore((s) => s.inspect);
  const raster = useMapStore((s) => s.raster);
  const close = useMapStore((s) => s.setInspect);
  const q = useSample(raster?.jobId, raster?.speciesKey, at);
  const pm = useProjectionMessages();
  const changeLabel = (c: number | null) =>
    c === 1 ? pm.unsuitable : c === 2 ? pm.lost : c === 3 ? pm.gained : c === 4 ? pm.stable : "—";

  if (!at) return null;
  return (
    <div className="w-60 text-sm" role="dialog" aria-label={msg.inspectLabel}>
      <div className="mb-1 flex items-start justify-between gap-2">
        <p className="num text-xs text-muted-foreground">{formatLngLat(at.lng, at.lat)}</p>
        <button
          type="button"
          className="-mt-1 -mr-1 rounded p-1 text-muted-foreground hover:bg-muted"
          onClick={() => close(null)}
          aria-label={msg.close}
        >
          <X className="size-3.5" aria-hidden />
        </button>
      </div>
      {q.isPending && (
        <div className="space-y-1.5" aria-busy="true">
          <Skeleton className="h-4 w-40" />
          <Skeleton className="h-4 w-32" />
        </div>
      )}
      {q.isError && <p className="text-destructive">{errorMessage(q.error)}</p>}
      {q.data && !q.data.in_area && <p className="text-muted-foreground">{msg.outsideArea}</p>}
      {q.data && q.data.in_area && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
          <dt className="text-muted-foreground">{msg.suitability}</dt>
          <dd className="num text-right">
            {formatScore(q.data.suitability)} <span className="text-muted-foreground">{msg.scoreUnit}</span>
          </dd>
          <dt className="text-muted-foreground">{msg.uncertainty}</dt>
          <dd className="num text-right">
            {formatDecimal(q.data.uncertainty, 3)} <span className="text-muted-foreground">{msg.sdUnit}</span>
          </dd>
          <dt className="text-muted-foreground">MESS</dt>
          <dd className="num text-right">{formatDecimal(q.data.mess, 1)}</dd>
          <dd className="col-span-2 text-xs">
            {q.data.mess === null
              ? msg.messNotAvailable
              : q.data.mess < 0
                ? msg.messOutside
                : msg.messInside}
          </dd>
          {q.data.effort !== undefined && q.data.effort !== null && (
            <>
              <dt className="text-muted-foreground">{msg.effort}</dt>
              <dd className="num text-right">
                {formatCount(q.data.effort)} <span className="text-muted-foreground">{msg.effortUnit}</span>
              </dd>
            </>
          )}
          {q.data.aoa_di !== undefined && q.data.aoa_di !== null && (
            <>
              <dt className="text-muted-foreground">{msg.aoaDi}</dt>
              <dd className="num text-right">
                {formatDecimal(q.data.aoa_di, 2)}
                {raster?.aoaThreshold !== undefined && (
                  <span className="text-muted-foreground"> ({msg.aoaThreshold(formatDecimal(raster.aoaThreshold, 2))})</span>
                )}
              </dd>
              {raster?.aoaThreshold !== undefined && (
                <dd className="col-span-2 text-xs">{q.data.aoa_di > raster.aoaThreshold ? msg.aoaOutside : msg.aoaInside}</dd>
              )}
            </>
          )}
          {(q.data.projections ?? []).map((p) => (
            <Fragment key={`${p.period}_${p.ssp}`}>
              <dt className="col-span-2 mt-1 border-t pt-1 text-xs font-semibold text-muted-foreground">
                <span className="num">{pm.inspectTitle(p.period, pm[`ssp_${p.ssp}`])}</span>
              </dt>
              {p.mean === null ? (
                <dd className="col-span-2 text-xs text-muted-foreground">{msg.messNotAvailable}</dd>
              ) : (
                <>
                  <dt className="text-muted-foreground">{pm.inspectMean}</dt>
                  <dd className="num text-right">{formatScore(p.mean)}</dd>
                  <dt className="text-muted-foreground">{pm.inspectSd}</dt>
                  <dd className="num text-right">{formatDecimal(p.sd, 3)}</dd>
                  <dd className="col-span-2 text-xs">
                    <span className="text-muted-foreground">{pm.inspectChange}:</span> {changeLabel(p.change)}
                    {p.agreement !== null && <span className="text-muted-foreground"> · {pm.inspectAgree(p.agreement, p.gcms)}</span>}
                  </dd>
                  {p.mess !== null && p.mess < 0 && <dd className="col-span-2 text-xs text-state-warning">{pm.inspectNovel}</dd>}
                </>
              )}
            </Fragment>
          ))}
        </dl>
      )}
    </div>
  );
}
