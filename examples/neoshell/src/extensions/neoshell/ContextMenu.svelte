<script lang="ts">
  import { Menu, Popover } from '@neoworks/svelte-gpui/ui'
  import type { MenuItem as LibraryItem } from '@neoworks/svelte-gpui/ui'
  import { stringOf } from '../../lib/record'
  import type { MenuItem, MenuState } from './desktopStore.svelte'

  // The desktop's right-click menu: the library menu opened at the pointer.
  // The library's items have no submenus, so choosing a row that has children
  // replaces the menu's rows with them, under a row that goes back.

  const SUBMENU_MARK = '›'
  const BACK_MARK = '‹'
  const MAX_HEIGHT = 480
  const MENU_WIDTH = 208

  interface Level {
    title: string
    items: MenuItem[]
  }

  let { menu, onclose }: { menu: MenuState; onclose: () => void } = $props()

  let trail = $state<Level[]>([])
  let descending = false

  const level = $derived(currentLevel(menu, trail))
  const rows = $derived(rowsOf(level, trail.length > 0))

  function currentLevel(state: MenuState, path: Level[]): Level {
    if (path.length === 0) {
      return { title: '', items: state.items }
    }
    return path[path.length - 1]
  }

  function rowsOf(shown: Level, nested: boolean): LibraryItem[] {
    const mapped = shown.items.map(libraryItemOf)
    if (!nested) {
      return mapped
    }
    const back: LibraryItem = { label: `${BACK_MARK}  ${shown.title}`, value: 'back' }
    return [back, { separator: true }, ...mapped]
  }

  function libraryItemOf(item: MenuItem): LibraryItem {
    if (item.separator === true) {
      return { separator: true }
    }
    const row: LibraryItem = {
      label: item.label,
      checked: item.checked,
      danger: item.danger,
      disabled: item.disabled,
      value: item,
    }
    if (item.children !== undefined) {
      row.shortcut = SUBMENU_MARK
    }
    return row
  }

  function choose(row: LibraryItem): void {
    if (row.value === 'back') {
      descending = true
      trail = trail.slice(0, -1)
      return
    }
    const item = row.value as MenuItem
    if (item.children !== undefined) {
      descending = true
      trail = [...trail, { title: stringOf(item.label), items: item.children }]
      return
    }
    if (item.action !== undefined) {
      item.action()
    }
  }

  function afterChoice(close: () => void): void {
    if (descending) {
      descending = false
      return
    }
    close()
  }
</script>

<Popover open at={[menu.x, menu.y]} side="bottom" align="start" offset={0} {onclose}>
  {#snippet children({ close })}
    {#key trail.length}
      <Menu items={rows} width={MENU_WIDTH} maxHeight={MAX_HEIGHT} onselect={choose} onclose={() => afterChoice(() => close('select'))} />
    {/key}
  {/snippet}
</Popover>
