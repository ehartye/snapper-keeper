import type { ReactNode } from 'react';

interface SettingsSectionProps {
  title: string;
  children: ReactNode;
}

export function SettingsSection({ title, children }: SettingsSectionProps) {
  return (
    <section>
      <h2 className="font-display text-sm mb-3">{title}</h2>
      <div className="bg-surface rounded-xl border border-border px-4 divide-y divide-border">
        {children}
      </div>
    </section>
  );
}
