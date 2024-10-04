import { initChanges, addChange, removeChange, getChanges, getChangesByBranch } from './binding.js'
import util from 'node:util'

const log = (() => {
  const log = (...values) => {
    console.log(
      ...values.map((value) =>
        util.inspect(value, {
          colors: true,
          depth: null,
          getters: true,
          showHidden: false,
          ...log.options,
        }),
      ),
    )
  }
  log.options = {}
  return log
})()

const root = process.cwd()

log(initChanges(root))

log(addChange({ package: '@scope/foo', releaseAs: 'patch' }, ['int'], root))

log(getChanges(root))

log(getChangesByBranch('feature/next', root))

log(removeChange('feature/next', root))
