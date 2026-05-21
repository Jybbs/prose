# Alignment Rules

The alignment rules align a shared token across consecutive rows so columns of assignments, annotations, and patterns read as vertical groups. The shared math lives in the [[aligner]] primitive, and each rule supplies the walker that decides what counts as a group. The per-rule `max-shift` knob caps how far rows shift to align, with `max-shift-policy` *(`split` / `drop` / `skip`)* deciding how a group whose widest member exceeds the cap resolves.

<RuleCardGrid family="alignment" />

For the per-rule knobs, see the [**Configuration**](/reference/configuration) reference. For the math the rules share, see the [[aligner]] primitive. The [[colon-targets]] walker drives the five `:` contexts the alignment rules read against.
