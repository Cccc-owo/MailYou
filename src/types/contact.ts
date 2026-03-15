export interface Contact {
  id: string
  name: string
  emails: string[]
  phones: string[]
  notes?: string
  groupId?: string
  avatarPath?: string
  sourceAccountId?: string
  createdAt: string
  updatedAt: string
}

export interface ContactGroup {
  id: string
  name: string
}
