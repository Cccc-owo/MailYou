export interface WindowControlsBridge {
  minimize(): Promise<void>
  toggleMaximize(): Promise<boolean>
  close(): Promise<void>
  isMaximized(): Promise<boolean>
  openExternal(url: string): Promise<void>
  focus(): Promise<void>
  setBackgroundSyncInterval(minutes: number): Promise<void>
  exportPdf(html: string, fileName: string): Promise<boolean>
  openTextFile(
    filters: { name: string; extensions: string[] }[],
  ): Promise<{ content: string; fileName: string } | null>
  saveTextFile(
    content: string,
    suggestedName: string,
    filters: { name: string; extensions: string[] }[],
  ): Promise<boolean>
}
