// Only accepts handles for this harness's freshly spawned children. Group mode
// requires spawn({ detached: true }) on the supported macOS/Linux platforms.
export async function stopChild(child, { group = false, graceMs = 2000 } = {}) {
  if (!child?.pid) return; // Spawn failure has no process to reap.
  function signal(name) {
    if (!group && (child.exitCode !== null || child.signalCode !== null)) return;
    try {
      if (group) process.kill(-child.pid, name);
      else child.kill(name);
    } catch (error) {
      if (error.code !== 'ESRCH') throw error;
    }
  }
  async function waitForExit(limit) {
    if (child.exitCode !== null || child.signalCode !== null) return;
    let timer, listener;
    try {
      await new Promise(resolve => {
        listener = resolve;
        child.once('exit', listener);
        timer = setTimeout(resolve, limit);
      });
    } finally {
      clearTimeout(timer);
      child.removeListener('exit', listener);
    }
  }
  signal('SIGTERM');
  await waitForExit(graceMs);
  // A browser leader can exit while its profile-writing helpers remain alive.
  // Retain and stop the owned group even after the leader has been reaped.
  signal('SIGKILL');
  await waitForExit(graceMs);
  if (child.exitCode === null && child.signalCode === null) throw Error('Test child did not exit');
}
