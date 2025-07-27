export interface SortedItem {
  sortValue: string;
}

export class LexoSorter<T extends SortedItem> {
  constructor(public items: T[]) {
    this.items = this.items.map((item, index) => {
      item.sortValue = item.sortValue || this.generateLexoRank(index);
      return item;
    });
  }

  public reorder(movedItem: T, beforeItem: T | null): T[] {
    const movedIndex = this.items.findIndex((i) => i.sortValue === movedItem.sortValue);
    if (movedIndex === -1) { return this.items; }

    const list = [
      ...this.items.slice(0, movedIndex),
      ...this.items.slice(movedIndex + 1),
    ];

    const insertIndex = beforeItem === null
      ? list.length
      : list.findIndex((i) => i.sortValue === beforeItem.sortValue);

    if (insertIndex === -1) { return this.items; }

    const prev = insertIndex > 0 ? list[insertIndex - 1].sortValue : null;
    const next = insertIndex < list.length ? list[insertIndex]?.sortValue : null;

    const newSortValue = !prev && !next
      ? this.generateLexoRank(0)
      : !prev
        ? this.decrement(next!)
        : !next
          ? this.increment(prev)
          : this.average(prev, next);

    movedItem.sortValue = newSortValue;
    return [...list.slice(0, insertIndex), movedItem, ...list.slice(insertIndex)];
  }

  private generateLexoRank(index: number): string {
    // Generate lexorank-style string that can be sorted lexicographically
    const base = "abcdefghijklmnopqrstuvwxyz";
    const step = 1000000; // Large step to allow insertions
    const value = (index + 1) * step;

    let result = "";
    let remaining = value;

    while (remaining > 0) {
      result = base[remaining % base.length] + result;
      remaining = Math.floor(remaining / base.length);
    }

    return result.padStart(6, "a");
  }

  private increment(_sortValue: string): string {
    // Add a character to make it come after
    return _sortValue + "n";
  }

  private decrement(_sortValue: string): string {
    // Insert a character to make it come before
    return (
      _sortValue.slice(0, -1) +
      String.fromCharCode(_sortValue.charCodeAt(_sortValue.length - 1) - 1) +
      "n"
    );
  }

  private average(prev: string, next: string): string {
    // Find a string that sorts between prev and next
    const maxLength = Math.max(prev.length, next.length);
    const paddedPrev = prev.padEnd(maxLength, "a");
    const paddedNext = next.padEnd(maxLength, "z");

    let result = "";
    for (let i = 0; i < maxLength; i++) {
      const prevChar = paddedPrev.charCodeAt(i);
      const nextChar = paddedNext.charCodeAt(i);

      if (prevChar < nextChar - 1) {
        result += String.fromCharCode(Math.floor((prevChar + nextChar) / 2));
        break;
      } else if (prevChar < nextChar) {
        result += String.fromCharCode(prevChar);
        result += "n";
        break;
      } else {
        result += String.fromCharCode(prevChar);
      }
    }

    return result || prev + "n";
  }
}
