import { expect, it } from "vitest";
import { displayAudioLevel } from "./audio-level";

it("maps silence, quiet speech and full-scale samples to bounded display levels", () => {
  expect(displayAudioLevel(0)).toBe(0);
  expect(displayAudioLevel(0.00001)).toBe(0);
  expect(displayAudioLevel(0.001)).toBe(0);
  expect(displayAudioLevel(0.01)).toBeCloseTo(1 / 3);
  expect(displayAudioLevel(0.1)).toBeCloseTo(2 / 3);
  expect(displayAudioLevel(1)).toBe(1);
  expect(displayAudioLevel(5)).toBe(1);
  for (const invalid of [NaN, Infinity, -Infinity, -1]) {
    expect(displayAudioLevel(invalid)).toBe(0);
  }
});
