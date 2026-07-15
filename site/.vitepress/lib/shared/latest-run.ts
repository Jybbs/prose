// Supersedes overlapping async runs, leaving only the latest one live.
export function latestRun() {
  let generation = 0

  // Starts a run, superseding any earlier one, and returns the check that
  // run polls to abandon itself once a newer one begins.
  function begin(): () => boolean {
    const gen = ++generation
    return () => gen !== generation
  }

  // Supersedes the run in flight without starting one.
  function cancel(): void {
    generation += 1
  }

  return { begin, cancel }
}
