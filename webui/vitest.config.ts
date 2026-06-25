import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [
      { find: /^@\//, replacement: `${path.resolve(__dirname, 'src')}/` },
    ],
  },
  server: {
    fs: {
      allow: ['..'],
    },
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
});
