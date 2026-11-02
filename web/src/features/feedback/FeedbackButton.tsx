// "Feedback" on every signed-in page (ADR-0111): a side panel where writing, capturing the
// screen, adding or pasting an image and recording one's voice are each one click. The page,
// version and browser are attached automatically. The draft survives closing the panel.

import { Camera, CheckCircle2, ImagePlus, Lightbulb, MessageSquarePlus, Mic, MessageCircle, Square, TriangleAlert, X } from "lucide-react";
import { useRef, useState, type ClipboardEvent, type DragEvent } from "react";
import { useLocation } from "react-router";

import { errorMessage } from "@/api/client";
import { useMe, useSendFeedback } from "@/api/hooks";
import type { FeedbackKind } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { useLang } from "@/i18n/lang";

import { MAX_BYTES, MAX_FILES, canCapture, canRecord, captureScreen, clock, shrinkImage, toBase64, useRecorder, type Attachment } from "./media";
import { useFeedbackMessages } from "./messages";

const KINDS: { id: FeedbackKind; icon: typeof Lightbulb }[] = [
  { id: "problem", icon: TriangleAlert },
  { id: "idea", icon: Lightbulb },
  { id: "other", icon: MessageCircle },
];

const ACCEPT = "image/png,image/jpeg,image/webp,image/gif";

export function FeedbackButton({ className }: { className?: string }) {
  const m = useFeedbackMessages();
  const me = useMe();
  const lang = useLang((s) => s.lang);
  const location = useLocation();
  const send = useSendFeedback();
  const [open, setOpen] = useState(false);
  const [kind, setKind] = useState<FeedbackKind | null>(null);
  const [message, setMessage] = useState("");
  const [files, setFiles] = useState<Attachment[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [sent, setSent] = useState(false);
  const [capturing, setCapturing] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);

  const add = (a: Attachment[]) => {
    if (files.length + a.length > MAX_FILES) {
      a.forEach((x) => URL.revokeObjectURL(x.url));
      setProblem(m.tooMany(MAX_FILES));
      return;
    }
    setProblem(null);
    setFiles((prev) => [...prev, ...a]);
  };
  const removeFile = (f: Attachment) => {
    URL.revokeObjectURL(f.url);
    setFiles((prev) => prev.filter((x) => x.id !== f.id));
  };
  const recorder = useRecorder(
    (a) => add([a]),
    () => setProblem(m.micDenied),
  );
  // Hosted mode only: feedback needs an account (local mode has no one to read it).
  if (!me.data) return null;

  const addImages = async (list: Iterable<File>) => {
    const images = [...list].filter((f) => ACCEPT.split(",").includes(f.type));
    if (images.length) add(await Promise.all(images.map(shrinkImage)));
  };
  const onPaste = (e: ClipboardEvent) => {
    const images = [...e.clipboardData.files].filter((f) => f.type.startsWith("image/"));
    if (images.length) {
      e.preventDefault();
      void addImages(images);
    }
  };
  const onDrop = (e: DragEvent) => {
    if (e.dataTransfer.files.length) {
      e.preventDefault();
      void addImages(e.dataTransfer.files);
    }
  };
  const capture = async () => {
    setProblem(null);
    // The panel is hidden while the browser captures, so the screenshot shows the page.
    setCapturing(true);
    try {
      await new Promise((r) => setTimeout(r, 200));
      add([await captureScreen()]);
    } catch (err) {
      if (!(err instanceof DOMException && err.name === "NotAllowedError")) setProblem(m.captureFailed);
    } finally {
      setCapturing(false);
    }
  };

  const reset = () => {
    files.forEach((f) => URL.revokeObjectURL(f.url));
    setFiles([]);
    setMessage("");
    setKind(null);
    setProblem(null);
    setSent(false);
    send.reset();
  };

  const submit = async () => {
    setProblem(null);
    if (!message.trim() && files.length === 0) return setProblem(m.needContent);
    const total = files.reduce((n, f) => n + f.blob.size, 0);
    if (total > MAX_BYTES) return setProblem(m.tooLarge(MAX_BYTES >> 20));
    const attachments = await Promise.all(files.map(async (f) => ({ name: f.name, mime: f.mime, data: await toBase64(f.blob) })));
    const job = /^\/jobs\/([^/]+)/.exec(location.pathname)?.[1];
    send.mutate(
      {
        kind: kind ?? "other",
        message: message.trim(),
        page: location.pathname + location.search,
        context: {
          lang,
          screen: `${window.innerWidth}×${window.innerHeight} @${window.devicePixelRatio}x`,
          user_agent: navigator.userAgent,
          ...(job ? { job_id: job } : {}),
        },
        attachments,
      },
      { onSuccess: () => setSent(true) },
    );
  };

  const recording = recorder.seconds !== null;
  return (
    <>
      <Button variant="outline" size="sm" className={className} onClick={() => setOpen(true)} aria-label={m.buttonLabel} data-feedback-button>
        <MessageSquarePlus aria-hidden /> <span className="hidden sm:inline">{m.button}</span>
      </Button>
      <Sheet
        open={open && !capturing}
        onOpenChange={(o) => {
          if (recording) return;
          setOpen(o);
          // After a successful send, the next opening starts from an empty form.
          if (!o && sent) reset();
        }}
      >
        <SheetContent side="right" className="w-full gap-0 sm:max-w-md" closeLabel={m.closeLabel}>
          <SheetHeader className="pr-12">
            <SheetTitle>{m.title}</SheetTitle>
            <SheetDescription>{m.intro}</SheetDescription>
          </SheetHeader>
          {sent ? (
            <div className="flex flex-1 flex-col items-center justify-center gap-4 p-6 text-center" role="status">
              <CheckCircle2 className="size-10 text-state-success" aria-hidden />
              <p className="font-medium">{m.thanks}</p>
              <Button variant="outline" onClick={reset}>
                {m.another}
              </Button>
            </div>
          ) : (
            <>
              <div className="flex-1 space-y-4 overflow-y-auto px-4 pb-2" onDrop={onDrop} onDragOver={(e) => e.preventDefault()}>
                <fieldset className="space-y-1.5">
                  <legend className="mb-1.5 text-sm font-medium">{m.kindLabel}</legend>
                  <div className="flex flex-wrap gap-2">
                    {KINDS.map(({ id, icon: Icon }) => (
                      <Button
                        key={id}
                        type="button"
                        size="sm"
                        variant={kind === id ? "default" : "outline"}
                        aria-pressed={kind === id}
                        onClick={() => setKind(kind === id ? null : id)}
                      >
                        <Icon aria-hidden /> {m[id]}
                      </Button>
                    ))}
                  </div>
                </fieldset>
                <div className="space-y-1.5">
                  <label htmlFor="feedback-message" className="text-sm font-medium">
                    {m.messageLabel}
                  </label>
                  <textarea
                    id="feedback-message"
                    autoFocus
                    rows={6}
                    value={message}
                    onChange={(e) => setMessage(e.target.value)}
                    onPaste={onPaste}
                    placeholder={m.placeholder}
                    maxLength={10_000}
                    className="w-full resize-y rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
                  />
                </div>
                <div className="flex flex-wrap gap-2">
                  {canCapture() && (
                    <Button type="button" variant="outline" size="sm" onClick={() => void capture()} disabled={recording}>
                      <Camera aria-hidden /> {m.screenshot}
                    </Button>
                  )}
                  <Button type="button" variant="outline" size="sm" onClick={() => fileInput.current?.click()} disabled={recording}>
                    <ImagePlus aria-hidden /> {m.addImage}
                  </Button>
                  <input
                    ref={fileInput}
                    type="file"
                    accept={ACCEPT}
                    multiple
                    hidden
                    onChange={(e) => {
                      if (e.target.files) void addImages(e.target.files);
                      e.target.value = "";
                    }}
                  />
                  {canRecord() &&
                    (recording ? (
                      <Button type="button" variant="destructive" size="sm" onClick={recorder.stop}>
                        <Square aria-hidden /> {m.stop(clock(recorder.seconds ?? 0))}
                      </Button>
                    ) : (
                      <Button type="button" variant="outline" size="sm" onClick={() => void recorder.start()}>
                        <Mic aria-hidden /> {m.record}
                      </Button>
                    ))}
                </div>
                {recording && (
                  <p className="flex items-center gap-2 text-sm text-destructive" role="status">
                    <span className="size-2 animate-pulse rounded-full bg-destructive" aria-hidden /> {m.recording}
                  </p>
                )}
                <p className="text-xs text-muted-foreground">{m.pasteHint}</p>
                {files.length > 0 && (
                  <ul className="space-y-2">
                    {files.map((f) => (
                      <li key={f.id} className="flex items-center gap-2 rounded-lg border p-2">
                        {f.mime.startsWith("image/") ? (
                          <img src={f.url} alt={m.image} className="h-14 w-20 shrink-0 rounded object-cover object-top" />
                        ) : (
                          <audio src={f.url} controls className="h-9 min-w-0 flex-1" aria-label={m.voice} />
                        )}
                        {f.mime.startsWith("image/") && <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">{f.name}</span>}
                        <Button type="button" variant="ghost" size="icon-sm" onClick={() => removeFile(f)} aria-label={m.remove(f.name)}>
                          <X aria-hidden />
                        </Button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <SheetFooter className="border-t">
                <p className="text-xs text-muted-foreground">{m.contextNote}</p>
                {(problem ?? send.error) && (
                  <p role="alert" className="text-sm text-destructive">
                    {problem ?? errorMessage(send.error)}
                  </p>
                )}
                <Button onClick={() => void submit()} disabled={send.isPending || recording} className="w-full">
                  {send.isPending ? m.sending : m.send}
                </Button>
              </SheetFooter>
            </>
          )}
        </SheetContent>
      </Sheet>
    </>
  );
}
