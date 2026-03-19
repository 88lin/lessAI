import { memo } from "react";
import type { LucideIcon } from "lucide-react";
import { LoaderCircle } from "lucide-react";

interface ActionButtonProps {
  icon: LucideIcon;
  label: string;
  busy: boolean;
  disabled?: boolean;
  onClick: () => void;
  variant?: "primary" | "secondary" | "danger";
  block?: boolean;
  className?: string;
}

export const ActionButton = memo(function ActionButton({
  icon: Icon,
  label,
  busy,
  disabled = false,
  onClick,
  variant = "secondary",
  block = false,
  className
}: ActionButtonProps) {
  const classes = [
    "button",
    `button-${variant}`,
    block ? "button-block" : "",
    className ?? ""
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      className={classes}
      onClick={onClick}
      disabled={busy || disabled}
    >
      {busy ? <LoaderCircle className="spin" /> : <Icon />}
      {label}
    </button>
  );
});
