import { zxcvbn, zxcvbnOptions } from "@zxcvbn-ts/core";
import * as common from "@zxcvbn-ts/language-common";
import * as en from "@zxcvbn-ts/language-en";

// Configure zxcvbn once at module load. The dictionaries and adjacency graphs
// let it recognise dictionary words, l33t-speak, and keyboard walks. We do NOT
// wire up `translations`, because we render our own localized labels off the
// numeric score rather than zxcvbn's English feedback strings.
zxcvbnOptions.setOptions({
  dictionary: { ...common.dictionary, ...en.dictionary },
  graphs: common.adjacencyGraphs,
});

/** zxcvbn strength score: 0 (weakest) … 4 (strongest). */
export type Score = 0 | 1 | 2 | 3 | 4;

/** Minimum score required for a *master* password (Setup / Settings). */
export const MIN_MASTER_SCORE: Score = 2;

/** Estimate password strength. Synchronous; safe to call on each keystroke. */
export function scorePassword(password: string): Score {
  return zxcvbn(password).score as Score;
}
