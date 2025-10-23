import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
	plugins: [
		svelte({
			compilerOptions: {
				customElement: true
			}
		})
	],
	build: {
		emptyOutDir: false, // Don't empty the dist folder
		lib: {
			entry: 'src/lib/web-components.js',
			name: 'MatchboxWeb',
			fileName: (format) => `web-components/matchbox-web.${format}.js`,
			formats: ['es', 'umd']
		},
		rollupOptions: {
			external: ['argon2-browser'],
			output: {
				globals: {
					'argon2-browser': 'argon2'
				}
			}
		},
		outDir: 'dist'
	},
	define: {
		global: 'globalThis',
	},
	resolve: {
		alias: {
			buffer: 'buffer',
			process: 'process/browser',
		}
	}
});
