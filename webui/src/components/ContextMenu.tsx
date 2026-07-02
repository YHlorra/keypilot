import * as RadixDropdownMenu from "@radix-ui/react-dropdown-menu";
import { Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";

export interface ContextMenuProps {
  children: React.ReactNode;
  providerId: number;
  onDelete: (id: number) => void;
}

export const ContextMenu = ({ children, providerId, onDelete }: ContextMenuProps) => {
  return (
    <RadixDropdownMenu.Root>
      <RadixDropdownMenu.Trigger asChild onContextMenu={(e) => e.preventDefault()}>
        <div
          data-testid="context-menu-trigger"
          className="contents"
          onContextMenu={(e) => e.preventDefault()}
        >
          {children}
        </div>
      </RadixDropdownMenu.Trigger>

      <RadixDropdownMenu.Portal>
        <RadixDropdownMenu.Content
          data-testid="context-menu-content"
          sideOffset={4}
          className={cn(
            "min-w-[160px] rounded-[8px] border border-[var(--color-border)]",
            "bg-[var(--color-surface)] p-1 shadow-md",
            "z-50"
          )}
        >
          <RadixDropdownMenu.Item
            data-testid="delete-item"
            onClick={() => onDelete(providerId)}
            className={cn(
              "flex items-center gap-2 px-3 py-2 rounded-[6px]",
              "text-sm text-destructive cursor-pointer",
              "outline-none hover:bg-[var(--color-surface-sunken)]"
            )}
          >
            <Trash2 className="h-4 w-4" />
            Delete credential
          </RadixDropdownMenu.Item>
        </RadixDropdownMenu.Content>
      </RadixDropdownMenu.Portal>
    </RadixDropdownMenu.Root>
  );
};