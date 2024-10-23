const { getDefinedPackageManager } = require('./index')

const agent = getDefinedPackageManager();

console.assert(agent === 'pnpm', 'Simple test failed')

console.info(`Simple test passed: ${agent}`)
