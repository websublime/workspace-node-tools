import { globSync } from 'glob'
import nodeFs from 'node:fs'
import nodeUrl from 'node:url'
import nodePath from 'node:path'
import { defineBuildConfig } from 'unbuild'

export default defineBuildConfig({
  entries: ['src/index.ts'],
  alias: {
    '@src': nodePath.resolve(nodePath.dirname(nodeUrl.fileURLToPath(import.meta.url)), 'src'),
  },
  sourcemap: true,
  clean: true,
  declaration: true, // generate .d.ts files
  externals: [
    /workspace-tools\..*\.node/,
    /workspace-tools\..*\.wasm/,
    /@websublime\/workspace-tools.*/,
    /\.\/workspace-tools\.wasi\.cjs/,
  ],
  rollup: {
    emitCJS: true,
    cjsBridge: true,
    inlineDependencies: true,
    resolve: {
      exportConditions: ['node'],
    },
  },
  hooks: {
    'build:done': async () => {
      const binaryFiles = globSync(['./src/workspace-tools.*.node', './src/workspace-tools.*.wasm'], {
        absolute: true,
      })

      const wasiShims = globSync(['./src/*.wasi.js', './src/*.wasi.cjs', './src/*.mjs', './src/browser.js'], {
        absolute: true,
        ignore: ['./src/main.mjs'],
      })

      nodeFs.mkdirSync('./dist/shared', { recursive: true })

      // Move the binary file to dist
      for (const file of binaryFiles) {
        const fileName = nodePath.basename(file)
        console.log('[build:done] Copying', file, 'to ./dist/shared')
        nodeFs.copyFileSync(file, `./dist/shared/${fileName}`)
        console.log(`[build:done] Cleaning ${file}`)
        nodeFs.rmSync(file)
      }

      for (const file of wasiShims) {
        const fileName = nodePath.basename(file)
        console.log('[build:done] Copying', file, 'to ./dist/shared')
        nodeFs.copyFileSync(file, `./dist/shared/${fileName}`)
      }
    },
  },
})
