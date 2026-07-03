import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config';

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      globals: true,
      setupFiles: ['./vitest.setup.ts'],
      include: ['src/**/*.{test,spec}.{ts,tsx}'],
      css: false,
      server: {
        deps: {
          inline: [
            '@testing-library/react',
            '@testing-library/jest-dom',
            '@testing-library/user-event',
            '@tanstack/react-query',
            'react',
            'react-dom',
            'lucide-react',
          ],
        },
      },
    },
  })
);
