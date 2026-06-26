import * as React from "react";
import { FileJson, FileSpreadsheet, Loader2 } from "lucide-react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { useImportUsage } from "@/hooks/useUsage";
import { useToast } from "./Icon";
import type { ImportFormat, ImportResult } from "@/types/api";

interface ImportModalProps {
  open: boolean;
  onClose: () => void;
}

type State = "idle" | "importing" | "success" | "error";

export function ImportModal({ open, onClose }: ImportModalProps) {
  const [format, setFormat] = React.useState<ImportFormat>("jsonl");
  const [sourceHint, setSourceHint] = React.useState("");
  const [content, setContent] = React.useState("");
  const [result, setResult] = React.useState<ImportResult | null>(null);
  const [state, setState] = React.useState<State>("idle");

  const { showToast } = useToast();
  const importMutation = useImportUsage();

  const handleClose = React.useCallback(() => {
    // Reset state on close
    setFormat("jsonl");
    setSourceHint("");
    setContent("");
    setResult(null);
    setState("idle");
    onClose();
  }, [onClose]);

  const handleSubmit = React.useCallback(async () => {
    if (!content.trim()) {
      showToast("Please paste content to import", "error");
      return;
    }
    setState("importing");
    try {
      const res = await importMutation.mutateAsync({
        content: content.trim(),
        format,
        sourceHint: sourceHint.trim() || undefined,
      });
      setResult(res);
      setState("success");
      showToast(`Imported ${res.imported} record${res.imported !== 1 ? "s" : ""}`, "success");
    } catch {
      setState("error");
      showToast(importMutation.error?.message ?? "Import failed", "error");
      setState("idle");
    }
  }, [content, format, sourceHint, importMutation, showToast]);

  const handleReset = React.useCallback(() => {
    setContent("");
    setResult(null);
    setState("idle");
  }, []);

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="Import Usage Records"
      footer={
        state === "success" ? (
          <Button onClick={handleClose}>Close</Button>
        ) : (
          <>
            <Button variant="outline" onClick={handleClose} disabled={state === "importing"}>
              Cancel
            </Button>
            <Button onClick={handleSubmit} disabled={state === "importing"}>
              {state === "importing" && <Loader2 size={14} className="animate-spin" />}
              Import
            </Button>
          </>
        )
      }
    >
      {state === "success" && result ? (
        <ResultDisplay result={result} onReset={handleReset} />
      ) : (
        <div className="flex flex-col gap-4">
          {/* Format selector */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">Format</label>
            <div className="flex gap-4">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="format"
                  value="jsonl"
                  checked={format === "jsonl"}
                  onChange={() => setFormat("jsonl")}
                  className="accent-primary"
                />
                <FileJson size={16} />
                <span className="text-sm">JSONL</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="format"
                  value="csv"
                  checked={format === "csv"}
                  onChange={() => setFormat("csv")}
                  className="accent-primary"
                />
                <FileSpreadsheet size={16} />
                <span className="text-sm">CSV</span>
              </label>
            </div>
          </div>

          {/* Source hint */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium" htmlFor="source-hint">
              Source Hint{" "}
              <span className="text-xs text-muted-foreground">(optional)</span>
            </label>
            <input
              id="source-hint"
              type="text"
              value={sourceHint}
              onChange={(e) => setSourceHint(e.target.value)}
              placeholder="e.g. claude-code, codex, opencode"
              className="flex h-9 w-full rounded-sm border border-input bg-transparent px-3 py-1 text-body-md placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            />
          </div>

          {/* Content textarea */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium" htmlFor="import-content">
              Content
            </label>
            <textarea
              id="import-content"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder="Paste content here…"
              rows={10}
              className="flex w-full rounded-sm border border-input bg-transparent px-3 py-2 text-body-md font-mono placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 resize-none"
            />
          </div>
        </div>
      )}
    </Modal>
  );
}

function ResultDisplay({ result, onReset }: { result: ImportResult; onReset: () => void }) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">Import Complete</h3>
        <div className="flex flex-wrap gap-2">
          <span className="inline-flex items-center gap-1 rounded-pill bg-success/10 text-success text-xs px-3 py-1 border border-success/20">
            Imported: {result.imported}
          </span>
          {result.skipped > 0 && (
            <span className="inline-flex items-center gap-1 rounded-pill bg-amber-500/10 text-amber-500 text-xs px-3 py-1 border border-amber-500/20">
              Skipped: {result.skipped}
            </span>
          )}
          {result.errors.length > 0 && (
            <span className="inline-flex items-center gap-1 rounded-pill bg-danger/10 text-danger text-xs px-3 py-1 border border-danger/20">
              Errors: {result.errors.length}
            </span>
          )}
        </div>
      </div>

      {result.errors.length > 0 && (
        <div className="flex flex-col gap-2">
          <h4 className="text-xs font-medium text-muted-foreground">Errors</h4>
          <div className="flex flex-col gap-1 max-h-32 overflow-y-auto">
            {result.errors.map((err, i) => (
              <p key={i} className="text-xs text-danger font-mono">
                {err}
              </p>
            ))}
          </div>
        </div>
      )}

      <Button variant="outline" size="sm" onClick={onReset}>
        Import More
      </Button>
    </div>
  );
}