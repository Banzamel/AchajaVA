import { describe, expect, it } from 'vitest';
import { planIcon, planKey, planWhen, type PlanRow } from '../../../src/core/plan/format';

const row = (patch: Partial<PlanRow> = {}): PlanRow => ({ id: 1, kind: 'say', label: 'x', next: new Date('2026-11-06T08:05:00').getTime() / 1000, repeat: false, report_mtime: 0, ...patch });

describe('plan format', () => {
  it('shows icons and times', () => {
    expect(planIcon('job')).toBe('⚙');
    expect(planIcon('other')).toBe('•');
    expect(planWhen(row())).toBe('06.11 08:05');
    expect(planWhen(row({ repeat: true }))).toBe('06.11 08:05  ↻');
  });

  it('changes its key when the plan changes', () => {
    expect(planKey([row()])).not.toBe(planKey([row({ next: 1 })]));
  });
});
