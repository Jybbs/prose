// Returns the remainder of `value` under `modulus`, always non-negative,
// where `%` alone carries the sign of the dividend.
export function posMod(value: number, modulus: number): number {
  return ((value % modulus) + modulus) % modulus
}
