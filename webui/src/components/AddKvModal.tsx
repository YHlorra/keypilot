import * as React from "react";
import { useState, useCallback } from "react";
import { Modal } from "./Modal";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { useToast } from "./Icon";
import type { Visibility } from "@/types/api";

interface AddKvModalProps {
  open: boolean;
  onClose: () => void;
  onAdd: (key: string, value: string, visibility: Visibility) => void;
}

export const AddKvModal = React.memo(function AddKvModal({ open, onClose, onAdd }: AddKvModalProps) {
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");
  const [visibility, setVisibility] = useState<Visibility>("visible");
  const { showToast } = useToast();

  const handleAdd = useCallback(() => {
    if (!key.trim()) {
      showToast("请输入键", "error");
      return;
    }
    if (!value.trim()) {
      showToast("请输入值", "error");
      return;
    }
    onAdd(key.trim(), value.trim(), visibility);
    setKey("");
    setValue("");
    setVisibility("visible");
    onClose();
  }, [key, value, visibility, onAdd, onClose, showToast]);

  const handleClose = useCallback(() => {
    setKey("");
    setValue("");
    setVisibility("visible");
    onClose();
  }, [onClose]);

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title="添加字段"
      footer={
        <>
          <Button variant="ghost" onClick={handleClose}>
            取消
          </Button>
          <Button onClick={handleAdd}>添加</Button>
        </>
      }
    >
      <div className="space-y-4">
        <div>
          <label className="text-sm font-medium mb-1.5 block">键</label>
          <Input
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="api_key"
            className="font-mono"
          />
        </div>
        <div>
          <label className="text-sm font-medium mb-1.5 block">值</label>
          <Input
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="sk-..."
            className="font-mono"
          />
        </div>
        <div>
          <label className="text-sm font-medium mb-1.5 block">可见性</label>
          <Select value={visibility} onValueChange={(v) => setVisibility(v as Visibility)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="visible">可见</SelectItem>
              <SelectItem value="masked">隐藏</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
    </Modal>
  );
});
