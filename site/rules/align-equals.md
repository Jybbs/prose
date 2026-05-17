---
category: auto-fix
related : [align-colons, align-imports, match-case-align, singleton-rule]
---

# align-equals

A run of consecutive bindings sits at the same indentation, the eye walks down the page, and every `=` sign lands at a **different column**. The reader stops at each line to find where the assignment splits. *Align-equals* gathers those runs into a **single column**, so a stretch of bindings reads as a list of values rather than a stack of expressions.

The rule walks consecutive single-target assignments at the same indentation level, picking up type annotations when present and treating augmented assignments (*`+=`, `|=`*) and walrus operators (*`:=`*) as non-members. The same alignment also runs across consecutive annotated function-parameter default values, so a signature with several `param: type = default` entries aligns its `=` column the same way a stretch of module-level bindings does. A blank line, a comment line, or a non-assignment statement resets the group, leaving each contiguous run aligned in isolation. Once an alignment group lands, [[singleton-rule]] prunes any one-member residue so a lone binding reads as plain code.

## Configuration

<AlignmentConfig />

<RuleMotivation>
  <template #problem>
    A reader who walks a run of consecutive bindings sees a stack of expressions, each starting and ending at a different column. The eye lands on the `=` sign at one column, then jumps to a different column for the next line, then a third for the one after. The reader's attention spends on visual bookkeeping rather than on the values being assigned.
  </template>
  <template #fix>
    *Align-equals* gathers the run into a single column. The same three bindings now read as a list of names on the left and a parallel list of values on the right. The eye drops straight down the column of equals signs and skims the values without re-finding the separator on every line.
  </template>
</RuleMotivation>

## The Canonical Case

Three consecutive bindings with varying left-hand widths align on the `=` sign. The eye drops down the column of equals signs and reads the right-hand sides as a parallel list.

<Fixture rule="align_equals" case="basic" />

## More Examples

<Fixture rule="align_equals" case="blank_line" title="Blank Lines Reset the Group" />

<Fixture rule="align_equals" case="annotated" title="Type Annotations Sit Inside the Alignment" />

<Fixture rule="align_equals" case="augmented" title="Augmented Assignments Break the Run" />

<Fixture rule="align_equals" case="with_comments" title="Comments Reset the Group, Like Blank Lines" />

<Fixture rule="align_equals" case="chained" title="Chained Assignments Stay Un-Aligned" />

<Fixture rule="align_equals" case="parameter_defaults" title="Function-Parameter Defaults Align on the Same Column" />

<Fixture rule="align_equals" case="multi_line_annotation" title="Multi-Line Annotations Break the Alignment Run" />

<Fixture rule="align_equals" case="shift_limit_split" title="A Widest Member Past `max-shift` Splits the Group" />

<Fixture rule="align_equals" case="idempotent" title="Already-Aligned Source Is Left Alone" />

## Related

`=` is one of four separator surfaces the alignment engine runs across. [[align-colons]] covers the `:` separator on dictionary literals, dataclass and Pydantic fields, function-signature annotations, and docstring `Args:` blocks. [[align-imports]] covers the `import` keyword on `from ... import ...` runs and the `as` keyword on `import ... as ...` runs. [[match-case-align]] covers the post-pattern `:` on single-expression case bodies inside a `match`. The [[singleton-rule]] governs what happens on `:`-shaped surfaces when an alignment group resolves to a single member.

For the underlying motivation, the landing page's [**reading metaphor**](/) frames why aligned columns read better than the minimalist alternative.
