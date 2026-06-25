import * as React from "react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { ThemeToggle } from "./ThemeToggle";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export const SettingsModal = React.memo(function SettingsModal({ open, onClose }: SettingsModalProps) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title="设置"
      footer={
        <Button variant="ghost" onClick={onClose}>
          关闭
        </Button>
      }
    >
      <div className="space-y-6">
        {/* Theme */}
        <div>
          <h3 className="text-sm font-medium mb-3">主题</h3>
          <ThemeToggle />
        </div>

        {/* About */}
        <div>
          <h3 className="text-sm font-medium mb-3">关于</h3>
          <div className="text-sm text-muted-foreground space-y-1">
            <p>KeyPilot V0.1</p>
            <p className="text-xs">轻量级 API 凭证管理</p>
            <a
              href="https://github.com/keypilot/keypilot"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary hover:underline"
            >
              GitHub
            </a>
          </div>
        </div>
      </div>
    </Modal>
  );
});
