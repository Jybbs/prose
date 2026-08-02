# Formatting Rules

The formatting rules clear the small scaffolding that clutters a statement once its shape is settled, each one dropping a token, a blank line, or a construct the surrounding code no longer needs and leaving whatever the removal exposes to the layout rules that own it. Some weigh the line budget before folding what remains onto one line, so a removal that would overflow keeps its break. Each rewrite is narrower than a layout rule and more pervasive than an ordering rule, tidying what the eye reads without touching the structure it reads.

<RuleCardList family="formatting" />

For enabling or disabling any of these rules, see the [**Configuration**](/reference/configuration) reference. For the pipeline order they fire in, see the [**Pipeline Order**](/reference/pipeline-order) reference.
