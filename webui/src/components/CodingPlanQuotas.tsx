import * as React from "react";
import { useCodingPlanQuota } from "@/hooks/useCodingPlanQuota";
import type { QuotaTier } from "@/types/api";

interface CodingPlanQuotasProps {
  providerId: number;
}

interface ProgressBarProps {
  percent: number;
  active: boolean;
}

// Inactive tiers show as a flat grey bar; active tiers fill with primary.
// Width is clamped client-side so a bad upstream value can't break the grid.
const ProgressBar = React.memo(function ProgressBar({ percent, active }: ProgressBarProps) {
  const clamped = Math.max(0, Math.min(100, percent));
  return (
    <div className="h-2 w-full rounded-sm bg-muted overflow-hidden">
      <div
        className={active ? "h-full bg-primary transition-[width]" : "h-full bg-muted-foreground/40 transition-[width]"}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
});

function formatReset(ts: number): string {
  const d = new Date(ts);
  // Compact en-US format; locale-stable on the wire, presentation only.
  return d.toLocaleString();
}

const TierRow = React.memo(function TierRow({ tier }: { tier: QuotaTier }) {
  const remaining = tier.remaining_percent ?? 100;
  const isActive = tier.status === "active";
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium text-foreground">{tier.label}</span>
        <span className="text-muted-foreground font-mono tabular-nums">
          {remaining.toFixed(0)}% left
        </span>
      </div>
      <ProgressBar percent={remaining} active={isActive} />
      {tier.resets_at_ms != null && (
        <p
          className="text-muted-foreground"
          style={{ fontSize: "var(--font-size-2xs)" }}
        >
          Resets {formatReset(tier.resets_at_ms)}
        </p>
      )}
    </div>
  );
});

export const CodingPlanQuotas = React.memo(function CodingPlanQuotas({
  providerId,
}: CodingPlanQuotasProps) {
  const { data, isLoading, error } = useCodingPlanQuota(providerId);

  if (isLoading) {
    return (
      <div className="flex flex-col gap-2 text-sm text-muted-foreground">
        <span>Loading coding plan...</span>
        <div className="h-2 w-full rounded-sm bg-muted animate-pulse" />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="text-sm text-destructive">Failed to load coding plan</div>
    );
  }

  if (!data.success) {
    // credential_status is lowercase per Rust serde ("invalid"/"expired"/"unknown").
    if (data.credential_status === "invalid" || data.credential_status === "expired") {
      const label = data.credential_status === "expired" ? "expired" : "invalid";
      return (
        <div className="text-sm text-destructive">
          API key {label}
          {data.credential_message ? `: ${data.credential_message}` : ""}
        </div>
      );
    }
    return (
      <div className="text-sm text-destructive">
        {data.error ?? "Provider not supported"}
      </div>
    );
  }

  if (data.tiers.length === 0) {
    return <div className="text-sm text-muted-foreground">No active tiers</div>;
  }

  return (
    <div className="space-y-3">
      {data.tiers.map((tier, idx) => (
        <TierRow key={`${tier.kind}-${idx}`} tier={tier} />
      ))}
    </div>
  );
});
