// Reads the message off an `Error`, leaving any other thrown value as-is.
export function errorMessage(err: unknown): unknown {
  return Error.isError(err) ? err.message : err
}
