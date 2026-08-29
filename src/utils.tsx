import { closeMainWindow, Clipboard, getPreferenceValues, showToast, Toast } from "@raycast/api";
import {
  generateParagraphs,
  generateSentences,
  generateWords,
  safeLoremIpsumNumberArg,
  LOREM_IPSUM_MAX_NUMBER,
} from "./lib/lorem";

export { generateParagraphs, generateSentences, generateWords, safeLoremIpsumNumberArg, LOREM_IPSUM_MAX_NUMBER };

export const showError = async (msg: string) => {
  await closeMainWindow();
  await showToast(Toast.Style.Failure, msg);
};

export const produceOutput = async (content: string) => {
  const { action } = getPreferenceValues();

  await closeMainWindow();

  switch (action) {
    case "clipboard":
      await Clipboard.copy(content);
      await showToast(Toast.Style.Success, "Copied to clipboard! 📋");
      break;

    case "paste":
      await Clipboard.paste(content);
      await showToast(Toast.Style.Success, "Pasted to active app! 📝");
      break;

    case "pasteAndCopy":
      await Clipboard.paste(content);
      await Clipboard.copy(content);
      await showToast(Toast.Style.Success, "Pasted to active app and copied to clipboard! 📋");
      break;
  }
};
