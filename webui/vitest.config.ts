import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config';

export default mergeConfig(
  viteConfig,
  defineConfig({
    server: {
      // Allow vitest to resolve tests under ../.bd/tasks/ (one level up from webui/).
      fs: { allow: ['..'] },
    },
    test: {
      environment: 'jsdom',
      globals: true,
      setupFiles: ['./vitest.setup.ts'],
      include: [
        '../.bd/tasks/**/tests/**/*.{test,spec}.tsx',
        '../.bd/tasks/**/tests/**/*.{test,spec}.ts',
        'src/**/*.{test,spec}.{ts,tsx}',
      ],
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