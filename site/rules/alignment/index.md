# Alignment Rules

The alignment rules align a shared token across consecutive rows so columns of assignments, annotations, and patterns read as vertical groups. The shared math lives in the [[aligner]] primitive, and each rule supplies the walker that decides what counts as a group. The per-rule `max-shift` knob bounds how far rows shift to align, wherein each rule walks a run in source order and breaks a fresh group at the first row whose width spread would exceed the cap. `false` lifts the cap so a contiguous run folds into one column, and `0` forbids any shift. A `# prose: skip` on one row holds it out of its group while the rest align around it.

<RuleCardList family="alignment" />

For the per-rule knobs, see the [**Configuration**](/reference/configuration) reference. For the math the rules share, see the [[aligner]] primitive. The [[colon-targets]] walker drives the five `:` contexts the alignment rules read against.
