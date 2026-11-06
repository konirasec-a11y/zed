/**
 * Formats a URL string
 *
 * @param linkStr - String of the URL
 * @param config - Configuration passed to MermaidJS
 * @returns The formatted URL or `undefined`.
 */
function formatUrl(linkStr) {
  const url = linkStr.trim();
  if (!url) {
    return undefined;
  }
  return url;
}

/**
 * Runs a function
 *
 * @param functionName - A dot separated path to the function relative to the `window`
 * @param params - Parameters to pass to the function
 */
const runFunc = (functionName, ...params) => {
  const arrPaths = functionName.split(".");
  const len = arrPaths.length - 1;
  const fnName = arrPaths[len];
  let obj = window;
  for (let i = 0; i < len; i++) {
    obj = obj[arrPaths[i]];
    if (!obj) {
      log.error(`Function name: ${functionName} not found in window`);
      return;
    }
  }
  obj[fnName](...params);
};

const getEdgeId = (from, to, { counter = 0, prefix, suffix }, id) => {
  if (id) {
    return id;
  }
  return `${prefix ? `${prefix}_` : ""}${from}_${to}_${counter}${
    suffix ? `_${suffix}` : ""
  }`;
};

const isValidShape = (shape) => {
  return true;
};

/**
 * Resettable state storage.
 * @example
 * ```
 * const state = new ImperativeState(() => ({
 *   foo: undefined as string | undefined,
 *   bar: [] as number[],
 *   baz: 1 as number | undefined,
 * }));
 *
 * state.records.foo = "hi";
 * console.log(state.records.foo); // prints "hi";
 * state.reset();
 * console.log(state.records.foo); // prints "default";
 *
 * // typeof state.records:
 * // {
 * //   foo: string | undefined, // actual: undefined
 * //   bar: number[],           // actual: []
 * //   baz: number | undefined, // actual: 1
 * // }
 * ```
 */
class ImperativeState {
  /**
   * @param init - Function that creates the default state.
   */
  constructor(init) {
    this.init = init;
    this.records = this.init();
  }

  reset() {
    this.records = this.init();
  }
}

module.exports = {
  formatUrl,
  runFunc,
  getEdgeId,
  isValidShape,
  ImperativeState,
};
