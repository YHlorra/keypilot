import * as React from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  children: React.ReactNode;
  title?: string;
  footer?: React.ReactNode;
}

export const Modal = React.memo(function Modal({ open, onClose, children, title, footer }: ModalProps) {
  return (
    <Dialog.Root open={open} onOpenChange={(val) => !val && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-black/40 animate-in fade-in duration-200" />
        <Dialog.Content
          className={cn(
            // Mobile: inset-x-4 gives 16px breathing room from viewport edges (avoids the modal
            // touching the screen border on narrow phones). sm+: center via translate.
            "fixed top-1/2 z-50 -translate-y-1/2",
            "inset-x-4 sm:inset-x-auto sm:left-1/2 sm:-translate-x-1/2",
            "max-w-3xl rounded-sm border border-border shadow-[0_8px_24px_rgba(0,0,0,0.18)]",
            "focus:outline-none",
            "max-h-[90vh] flex flex-col"
          )}
          style={{ backgroundColor: "var(--color-surface-elevated)" }}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-border flex-shrink-0">
            {title ? (
              <Dialog.Title className="text-lg font-semibold">{title}</Dialog.Title>
            ) : (
              <div />
            )}
            <Dialog.Close asChild>
              <button
                type="button"
                onClick={onClose}
                className="p-1 rounded hover:bg-accent transition-colors"
                aria-label="Close"
              >
                <X size={16} />
              </button>
            </Dialog.Close>
          </div>

          {/* Body */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            {children}
          </div>

          {/* Footer */}
          {footer && (
            <div className="flex items-center justify-end gap-2 px-6 py-4 border-t border-border flex-shrink-0">
              {footer}
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
});