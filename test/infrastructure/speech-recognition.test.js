import { describe, it, expect } from 'vitest';
import { parseSpokenNumber } from '../../src/infrastructure/speech-recognition.js';

describe('parseSpokenNumber', () => {
  it('parses single digit words', () => {
    expect(parseSpokenNumber('one')).toBe(1);
    expect(parseSpokenNumber('five')).toBe(5);
    expect(parseSpokenNumber('nine')).toBe(9);
    expect(parseSpokenNumber('zero')).toBe(0);
  });

  it('parses teen words (ten through nineteen)', () => {
    expect(parseSpokenNumber('ten')).toBe(10);
    expect(parseSpokenNumber('thirteen')).toBe(13);
    expect(parseSpokenNumber('nineteen')).toBe(19);
  });

  it('parses tens (twenty, thirty, ... ninety)', () => {
    expect(parseSpokenNumber('twenty')).toBe(20);
    expect(parseSpokenNumber('fifty')).toBe(50);
    expect(parseSpokenNumber('ninety')).toBe(90);
  });

  it('parses compound (twenty three → 23)', () => {
    expect(parseSpokenNumber('twenty three')).toBe(23);
    expect(parseSpokenNumber('forty two')).toBe(42);
    expect(parseSpokenNumber('ninety nine')).toBe(99);
  });

  it('parses hyphenated (twenty-three → 23)', () => {
    expect(parseSpokenNumber('twenty-three')).toBe(23);
    expect(parseSpokenNumber('sixty-seven')).toBe(67);
  });

  it('parses hundreds (one hundred → 100)', () => {
    expect(parseSpokenNumber('one hundred')).toBe(100);
  });

  it('parses hundred and (one hundred and forty four → 144)', () => {
    expect(parseSpokenNumber('one hundred and forty four')).toBe(144);
    expect(parseSpokenNumber('one hundred and twenty three')).toBe(123);
  });

  it('parses digit strings ("13" → 13)', () => {
    expect(parseSpokenNumber('13')).toBe(13);
    expect(parseSpokenNumber('144')).toBe(144);
    expect(parseSpokenNumber('7')).toBe(7);
  });

  it('strips filler words (umm thirteen → 13)', () => {
    expect(parseSpokenNumber('umm thirteen')).toBe(13);
    expect(parseSpokenNumber('uh twenty three')).toBe(23);
    expect(parseSpokenNumber('um like forty two')).toBe(42);
  });

  it('strips phrases (I think its thirteen → 13)', () => {
    expect(parseSpokenNumber("I think it's thirteen")).toBe(13);
    expect(parseSpokenNumber('maybe twenty three')).toBe(23);
    expect(parseSpokenNumber('is it forty two')).toBe(42);
  });

  it('handles self-correction — takes last number (twelve no thirteen → 13)', () => {
    expect(parseSpokenNumber('twelve no thirteen')).toBe(13);
    expect(parseSpokenNumber('twenty wait thirty')).toBe(30);
    expect(parseSpokenNumber('five actually seven')).toBe(7);
    expect(parseSpokenNumber('ten i mean twelve')).toBe(12);
  });

  it('returns null for unparseable input', () => {
    expect(parseSpokenNumber('firteen')).toBeNull();
    expect(parseSpokenNumber('hello world')).toBeNull();
    expect(parseSpokenNumber('blah blah')).toBeNull();
  });

  it('returns null for empty string', () => {
    expect(parseSpokenNumber('')).toBeNull();
    expect(parseSpokenNumber(null)).toBeNull();
    expect(parseSpokenNumber(undefined)).toBeNull();
  });

  it('handles "a hundred" as 100', () => {
    expect(parseSpokenNumber('a hundred')).toBe(100);
    expect(parseSpokenNumber('a hundred and two')).toBe(102);
  });

  it('handles mixed words and digits (twenty 3 → 23)', () => {
    expect(parseSpokenNumber('twenty 3')).toBe(23);
  });

  it('handles "and" connector gracefully', () => {
    expect(parseSpokenNumber('one hundred and one')).toBe(101);
  });
});
