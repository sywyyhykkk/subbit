// Share one in-flight reload and recheck once for requests received while it ran.
export function createReloadQueue(load: (initial: boolean) => Promise<void>) {
  let running: Promise<void> | null = null;
  let pending = false;
  let initialPending = false;
  return function reload(initial = false): Promise<void> {
    pending = true;
    initialPending ||= initial;
    if (!running) {
      running = Promise.resolve().then(async () => {
        try {
          while (pending) {
            pending = false;
            const initial = initialPending;
            initialPending = false;
            await load(initial);
          }
        } finally {
          running = null;
        }
      });
    }
    return running;
  };
}
