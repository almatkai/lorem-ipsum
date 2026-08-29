import { LoremIpsum } from "lorem-ipsum";

export const LOREM_IPSUM_MAX_NUMBER = 1000;

export const loremIpsumOptions = {
  sentencesPerParagraph: {
    max: 8,
    min: 4,
  },
  wordsPerSentence: {
    max: 16,
    min: 4,
  },
};

export const generator = new LoremIpsum(loremIpsumOptions);

export const generateParagraphs = (count: number) => {
  return Array.from(Array(count))
    .map(() =>
      generator.generateSentences(
        Math.floor(
          Math.random() *
            (loremIpsumOptions.sentencesPerParagraph.max - loremIpsumOptions.sentencesPerParagraph.min + 1),
        ) + loremIpsumOptions.sentencesPerParagraph.min,
      ),
    )
    .join("\r\n\r\n");
};

export const generateSentences = (count: number) => {
  return generator.generateSentences(count);
};

export const generateWords = (count: number) => {
  return generator.generateWords(count);
};

export const safeLoremIpsumNumberArg = async (arg: string | undefined) => {
  if (!arg) {
    arg = "1";
  }

  try {
    const parseableNumber = parseInt(arg, 10);

    if (isNaN(parseableNumber) || parseableNumber > LOREM_IPSUM_MAX_NUMBER) {
      return {
        error: {
          message: `Please enter a valid integer number, no more than ${LOREM_IPSUM_MAX_NUMBER}`,
        },
        safeLoremIpsumNumber: null as number | null,
      };
    }

    return {
      error: null,
      safeLoremIpsumNumber: parseableNumber,
    };
  } catch {
    return {
      error: {
        message: "Something went wrong",
      },
      safeLoremIpsumNumber: null as number | null,
    };
  }
};
