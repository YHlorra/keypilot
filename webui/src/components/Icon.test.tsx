import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ProviderIcon } from './Icon';

// Regression guard for the edit-credential crash:
// `ProviderIcon` previously crashed with "Cannot read properties of undefined
// (reading 'charAt')" whenever `preset` was a truthy custom string not present
// in PRESET_LABELS (e.g. user-typed "openrouter" via the "自定义…" picker).
describe('ProviderIcon', () => {
  it('renders the label initial for a known preset', () => {
    render(<ProviderIcon preset="openai" name="My OpenAI" />);
    expect(screen.getByTitle('OpenAI').textContent).toBe('O');
  });

  it('falls back to the provider name for an unknown custom preset', () => {
    // Before the fix: this threw TypeError because PRESET_LABELS["openrouter"]
    // is undefined and label.charAt(0) crashed.
    render(<ProviderIcon preset="openrouter" name="OpenRouter" />);
    expect(screen.getByTitle('OpenRouter').textContent).toBe('O');
  });

  it('falls back to the provider name when preset is null', () => {
    render(<ProviderIcon preset={null} name="My Custom" />);
    expect(screen.getByTitle('My Custom').textContent).toBe('M');
  });

  it('renders an <img> when icon is set, with src and alt derived from the path and name', () => {
    render(
      <ProviderIcon
        preset="openai"
        name="My OpenAI"
        icon="/icons/providers/openai.svg"
      />
    );
    const img = screen.getByRole('img', { name: 'My OpenAI' });
    expect(img).toHaveAttribute('src', '/icons/providers/openai.svg');
  });
});
