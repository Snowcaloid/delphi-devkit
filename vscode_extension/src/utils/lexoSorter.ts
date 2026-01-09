import { LexoRank } from "lexorank";

export interface SortedItem {
  sortValue: string;
}

export class LexoSorter<T extends SortedItem> {
  constructor(public items: T[]) {
    this.initializeSortValues();
    this.items = this.items.sort((a, b) => a.sortValue.localeCompare(b.sortValue));
  }

  public reorder(movedItem: T, beforeItem: T | null): T[] {
    const movedIndex = this.items.findIndex((i) => i.sortValue === movedItem.sortValue);
    if (movedIndex < 0) return this.items;
    const insertIndex = beforeItem === null ? this.items.length : this.items.findIndex((i) => i.sortValue === beforeItem.sortValue);
    const list = [...this.items.slice(0, movedIndex), ...this.items.slice(movedIndex + 1)];
    if (insertIndex < 0) return this.items;
    movedItem.sortValue = this.calculateSortValueAtPosition(insertIndex, list);
    this.items = [...list.slice(0, insertIndex), movedItem, ...list.slice(insertIndex)];
    return this.items;
  }

  private initializeSortValues(): void {
    const itemsWithSortValues = this.items.filter(item => item.sortValue);
    const itemsWithoutSortValues = this.items.filter(item => !item.sortValue);
    if (itemsWithSortValues.length === 0)
      this.generateInitialSortValues();
    else if (itemsWithoutSortValues.length > 0)
      this.fillMissingSortValues();
  }

  private generateInitialSortValues(): void {
    let currentRank = LexoRank.middle();
    
    for (let i = 0; i < this.items.length; i++) {
      this.items[i].sortValue = currentRank.toString();
      if (i < this.items.length - 1)
        currentRank = currentRank.genNext();
    }
  }

  private fillMissingSortValues(): void {
    for (let i = 0; i < this.items.length; i++)
      if (!this.items[i].sortValue) {
        const prevValue = this.findPreviousSortValue(i);
        const nextValue = this.findNextSortValue(i);
        
        this.items[i].sortValue = this.calculateValueBetween(prevValue, nextValue);
      }
  }

  private findPreviousSortValue(index: number): string | null {
    for (let i = index - 1; i >= 0; i--)
      if (this.items[i].sortValue)
        return this.items[i].sortValue;
    return null;
  }

  private findNextSortValue(index: number): string | null {
    for (let i = index + 1; i < this.items.length; i++)
      if (this.items[i].sortValue)
        return this.items[i].sortValue;
    return null;
  }

  private calculateSortValueAtPosition(targetIndex: number, list: T[]): string {
    const prevValue = targetIndex > 0 ? list[targetIndex - 1].sortValue : null;
    const nextValue = targetIndex < list.length ? list[targetIndex].sortValue : null;

    return this.calculateValueBetween(prevValue, nextValue);
  }

  private calculateValueBetween(prevValue: string | null, nextValue: string | null): string {
    if (!prevValue && !nextValue)
      return LexoRank.middle().toString();
    else if (!prevValue)
      return LexoRank.parse(nextValue!).genPrev().toString();
    else if (!nextValue)
      return LexoRank.parse(prevValue).genNext().toString();
    else {
      // Both values exist, generate between them
      const prevRank = LexoRank.parse(prevValue);
      const nextRank = LexoRank.parse(nextValue);
      return prevRank.between(nextRank).toString();
    }
  }
}
