import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AuditBadge from './AuditBadge';
import type { AuditFinding } from '../api/tauri-bridge';

const audit = vi.hoisted(() => ({ current: [] as AuditFinding[] }));
vi.mock('../hooks/useAudit', () => ({ useAudit: () => audit.current }));

function finding(severity: AuditFinding['severity'], skill = 'pdf'): AuditFinding {
  return {
    skill,
    kind: 'skill',
    severity,
    message: `${severity} issue`,
    file: 'SKILL.md',
    line: 3,
  };
}

function LocationProbe() {
  const { pathname } = useLocation();
  return <span data-testid="location">{pathname}</span>;
}

function renderBadge() {
  return render(
    <MemoryRouter>
      <AuditBadge />
      <LocationProbe />
    </MemoryRouter>
  );
}

afterEach(() => {
  audit.current = [];
});

describe('AuditBadge', () => {
  it('is hidden without findings', () => {
    renderBadge();
    expect(screen.queryByRole('button', { name: /finding|risk/ })).not.toBeInTheDocument();
  });

  it('counts high and critical findings', () => {
    audit.current = [finding('CRITICAL'), finding('HIGH'), finding('LOW')];
    renderBadge();
    expect(screen.getByRole('button')).toHaveTextContent('2 high risk');
  });

  it('shows low and medium findings without a high risk label', () => {
    audit.current = [finding('MEDIUM'), finding('LOW')];
    renderBadge();
    expect(screen.getByRole('button')).toHaveTextContent('2 findings');
  });

  it('lists medium and above, and counts low findings', () => {
    audit.current = [finding('HIGH'), finding('MEDIUM'), finding('LOW'), finding('LOW')];
    renderBadge();
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getAllByRole('link').map((l) => l.textContent)).toEqual([
      'HIGH pdfHIGH issue — SKILL.md:3',
      'MEDIUM pdfMEDIUM issue — SKILL.md:3',
      '2 low findings · Open the audit page',
    ]);
  });

  it('opens the finding’s skill in the Web UI', () => {
    audit.current = [finding('HIGH', 'frontend/doctor')];
    renderBadge();
    fireEvent.click(screen.getByRole('button'));
    fireEvent.click(screen.getByRole('link', { name: /frontend\/doctor/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/skills/frontend%2Fdoctor');
  });

  it('opens the Web UI audit page', () => {
    audit.current = [finding('LOW')];
    renderBadge();
    fireEvent.click(screen.getByRole('button'));
    fireEvent.click(screen.getByRole('link', { name: /audit page/ }));
    expect(screen.getByTestId('location')).toHaveTextContent('/audit');
  });
});
