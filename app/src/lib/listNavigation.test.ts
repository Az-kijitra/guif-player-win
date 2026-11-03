import { describe, expect, it } from "vitest";
import { nextListIndex } from "./listNavigation";

describe("nextListIndex in a list", () => {
  it("moves one item with the up and down arrows", () => {
    expect(nextListIndex("ArrowDown", 2, 5)).toBe(3);
    expect(nextListIndex("ArrowUp", 2, 5)).toBe(1);
  });

  it("stops at both ends", () => {
    expect(nextListIndex("ArrowDown", 4, 5)).toBe(4);
    expect(nextListIndex("ArrowUp", 0, 5)).toBe(0);
  });

  it("goes to the ends with Home and End", () => {
    expect(nextListIndex("Home", 3, 5)).toBe(0);
    expect(nextListIndex("End", 1, 5)).toBe(4);
  });

  it("moves a page and stops at the ends", () => {
    expect(nextListIndex("PageDown", 5, 100, { pageSize: 20 })).toBe(25);
    expect(nextListIndex("PageDown", 95, 100, { pageSize: 20 })).toBe(99);
    expect(nextListIndex("PageUp", 5, 100, { pageSize: 20 })).toBe(0);
  });

  it("leaves the left and right arrows alone", () => {
    expect(nextListIndex("ArrowLeft", 2, 5)).toBeNull();
    expect(nextListIndex("ArrowRight", 2, 5)).toBeNull();
  });

  it("ignores other keys and empty lists", () => {
    expect(nextListIndex("Enter", 2, 5)).toBeNull();
    expect(nextListIndex("ArrowDown", 0, 0)).toBeNull();
  });
});

describe("nextListIndex in a grid", () => {
  const grid = { columns: 4 };

  it("moves a row with the up and down arrows", () => {
    expect(nextListIndex("ArrowDown", 1, 10, grid)).toBe(5);
    expect(nextListIndex("ArrowUp", 5, 10, grid)).toBe(1);
  });

  it("moves one item with the left and right arrows", () => {
    expect(nextListIndex("ArrowRight", 1, 10, grid)).toBe(2);
    expect(nextListIndex("ArrowLeft", 1, 10, grid)).toBe(0);
  });

  it("goes to the last item when the row below is short", () => {
    expect(nextListIndex("ArrowDown", 7, 10, grid)).toBe(9);
  });

  it("stays put on the first row with the up arrow", () => {
    expect(nextListIndex("ArrowUp", 2, 10, grid)).toBe(0);
  });
});
