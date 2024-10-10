import test from 'ava';

import {
  getDefinedPackageManager,
  PackageManager
} from '../index';

test('get defined package manager', (t) => {
  t.is(getDefinedPackageManager(), PackageManager.Pnpm);
});
