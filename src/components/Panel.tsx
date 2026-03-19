import { memo } from "react";
import type { PanelProps } from "../lib/constants";

export const Panel = memo(function Panel({
  title,
  subtitle,
  action,
  footer,
  className,
  bodyClassName,
  children
}: PanelProps) {
  return (
    <section className={`panel ${className ?? ""}`.trim()}>
      <header className="panel-header">
        <div className="panel-heading">
          {subtitle ? <p className="panel-subtitle">{subtitle}</p> : null}
          <h2>{title}</h2>
        </div>
        {action ? <div className="panel-action">{action}</div> : null}
      </header>
      <div className={`panel-body ${bodyClassName ?? ""}`.trim()}>{children}</div>
      {footer ? <footer className="panel-footer">{footer}</footer> : null}
    </section>
  );
});
