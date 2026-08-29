/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** Default Action - What you would like to happen when running the command */
  "action": "clipboard" | "paste" | "pasteAndCopy"
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `paragraphs` command */
  export type Paragraphs = ExtensionPreferences & {}
  /** Preferences accessible in the `sentences` command */
  export type Sentences = ExtensionPreferences & {}
  /** Preferences accessible in the `words` command */
  export type Words = ExtensionPreferences & {}
  /** Preferences accessible in the `ai-generate` command */
  export type AiGenerate = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `paragraphs` command */
  export type Paragraphs = {
  /** No. of paragraphs */
  "numberOfParagraphs": string
}
  /** Arguments passed to the `sentences` command */
  export type Sentences = {
  /** No. of sentences */
  "numberOfSentences": string
}
  /** Arguments passed to the `words` command */
  export type Words = {
  /** No. of words */
  "numberOfWords": string
}
  /** Arguments passed to the `ai-generate` command */
  export type AiGenerate = {
  /** any topic */
  "topic": string
}
}

