import { describe, expect, it } from "vitest";
import { normalize, matchScore } from "./search";

describe("normalize", () => {
  it("lowercases the text", () => {
    expect(normalize("Beethoven")).toBe("beethoven");
  });

  it("removes punctuation", () => {
    expect(normalize("rock 'n' roll")).toBe("rocknroll");
  });

  it("removes straight apostrophes", () => {
    expect(normalize("Don't Stop")).toBe("dontstop");
  });

  it("removes curly apostrophes", () => {
    expect(normalize("Don’t Stop")).toBe("dontstop");
  });

  it("strips ellipsis characters", () => {
    expect(normalize("wait...")).toBe("wait");
  });

  it("keeps letters and digits", () => {
    expect(normalize("AC/DC 1979")).toBe("acdc1979");
  });

  it("keeps unicode letters", () => {
    expect(normalize("Björk")).toBe("björk");
  });

  it("returns empty string for empty input", () => {
    expect(normalize("")).toBe("");
  });

  it("returns empty string for punctuation-only input", () => {
    expect(normalize("...---")).toBe("");
  });
});

describe("matchScore", () => {
  it("returns 0 for an exact match", () => {
    expect(matchScore("beethoven", "beethoven")).toBe(0);
  });

  it("returns 1 for a prefix match", () => {
    expect(matchScore("beethoven", "beet")).toBe(1);
  });

  it("returns 2 for a substring match that is not a prefix", () => {
    expect(matchScore("beethoven", "hoven")).toBe(2);
  });

  it("returns Infinity when the query is absent from the text", () => {
    expect(matchScore("beethoven", "mozart")).toBe(Infinity);
  });

  it("exact match scores lower than prefix match", () => {
    expect(matchScore("a", "a")).toBeLessThan(matchScore("ab", "a"));
  });

  it("prefix match scores lower than interior substring match", () => {
    expect(matchScore("abc", "a")).toBeLessThan(matchScore("bca", "a"));
  });

  it("empty query matches everything as exact", () => {
    expect(matchScore("", "")).toBe(0);
    expect(matchScore("beethoven", "")).toBe(1);
  });
});
