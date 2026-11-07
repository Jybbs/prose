// Calls each key handler in order, the JSX form of a template's several
// `@keydown` modifiers on one element.
export const keyHandlers = (...handlers: ((event: KeyboardEvent) => unknown)[]) =>
  (event: KeyboardEvent): void => {
    for (const handler of handlers) handler(event)
  }
