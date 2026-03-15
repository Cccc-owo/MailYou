import type { Contact } from '@/types/contact'

// ── vCard 3.0 ──

export function parseVCard(text: string): Partial<Contact>[] {
  const contacts: Partial<Contact>[] = []
  const blocks = text.split(/(?=BEGIN:VCARD)/i)

  for (const block of blocks) {
    if (!block.trim() || !/BEGIN:VCARD/i.test(block)) continue

    const contact: Partial<Contact> & { emails?: string[]; phones?: string[] } = {}
    const lines = unfoldVCardLines(block)

    for (const line of lines) {
      const [key, ...rest] = line.split(':')
      const value = rest.join(':').trim()
      if (!value) continue

      const baseKey = key.split(';')[0].toUpperCase()
      switch (baseKey) {
        case 'FN':
          contact.name = value
          break
        case 'EMAIL':
          if (!contact.emails) contact.emails = []
          contact.emails.push(value)
          break
        case 'TEL':
          if (!contact.phones) contact.phones = []
          contact.phones.push(value)
          break
        case 'NOTE':
          contact.notes = value
          break
      }
    }

    if (contact.name || contact.emails?.length) {
      contacts.push(contact)
    }
  }

  return contacts
}

function unfoldVCardLines(block: string): string[] {
  // vCard line folding: a CRLF followed by a space means continuation
  return block.replace(/\r\n[ \t]/g, '').replace(/\n[ \t]/g, '').split(/\r?\n/)
}

function escapeVCardValue(value: string): string {
  return value.replace(/\\/g, '\\\\').replace(/;/g, '\\;').replace(/,/g, '\\,').replace(/\n/g, '\\n')
}

export function generateVCard(contacts: Contact[]): string {
  return contacts
    .map((c) => {
      const lines = ['BEGIN:VCARD', 'VERSION:3.0']
      lines.push(`FN:${escapeVCardValue(c.name || c.emails[0] || '')}`)
      for (const email of c.emails) {
        lines.push(`EMAIL:${email}`)
      }
      for (const phone of c.phones) {
        lines.push(`TEL:${phone}`)
      }
      if (c.notes) lines.push(`NOTE:${escapeVCardValue(c.notes)}`)
      lines.push('END:VCARD')
      return lines.join('\r\n')
    })
    .join('\r\n')
}

// ── CSV ──

export function parseCsv(text: string): Partial<Contact>[] {
  const lines = splitCsvLines(text)
  if (lines.length < 2) return []

  const header = parseCsvRow(lines[0]).map((h) => h.toLowerCase().trim())
  const nameIdx = header.indexOf('name')
  const emailIdx = header.indexOf('email')
  const phoneIdx = header.indexOf('phone')
  const notesIdx = header.indexOf('notes')
  const groupIdx = header.indexOf('group')

  const contacts: Partial<Contact>[] = []

  for (let i = 1; i < lines.length; i++) {
    if (!lines[i].trim()) continue
    const cols = parseCsvRow(lines[i])
    const contact: Partial<Contact> = {}
    if (nameIdx >= 0) contact.name = cols[nameIdx] ?? ''
    if (emailIdx >= 0) {
      const email = cols[emailIdx]?.trim()
      contact.emails = email ? [email] : []
    }
    if (phoneIdx >= 0) {
      const phone = cols[phoneIdx]?.trim()
      contact.phones = phone ? [phone] : []
    }
    if (notesIdx >= 0) contact.notes = cols[notesIdx] || undefined
    // groupIdx is parsed but groupId mapping is left to the caller
    void groupIdx

    if (contact.name || contact.emails?.length) {
      contacts.push(contact)
    }
  }

  return contacts
}

function splitCsvLines(text: string): string[] {
  // Split by newlines that are NOT inside quoted fields
  const lines: string[] = []
  let current = ''
  let inQuotes = false

  for (let i = 0; i < text.length; i++) {
    const ch = text[i]
    if (ch === '"') {
      inQuotes = !inQuotes
      current += ch
    } else if ((ch === '\n' || ch === '\r') && !inQuotes) {
      if (ch === '\r' && text[i + 1] === '\n') i++
      lines.push(current)
      current = ''
    } else {
      current += ch
    }
  }
  if (current) lines.push(current)
  return lines
}

function parseCsvRow(line: string): string[] {
  const cols: string[] = []
  let current = ''
  let inQuotes = false

  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (ch === '"') {
      if (inQuotes && line[i + 1] === '"') {
        current += '"'
        i++
      } else {
        inQuotes = !inQuotes
      }
    } else if (ch === ',' && !inQuotes) {
      cols.push(current)
      current = ''
    } else {
      current += ch
    }
  }
  cols.push(current)
  return cols
}

function escapeCsvField(value: string): string {
  if (value.includes(',') || value.includes('"') || value.includes('\n')) {
    return `"${value.replace(/"/g, '""')}"`
  }
  return value
}

export function generateCsv(contacts: Contact[]): string {
  const rows = ['Name,Email,Phone,Notes']
  for (const c of contacts) {
    rows.push(
      [
        escapeCsvField(c.name || ''),
        escapeCsvField(c.emails[0] || ''),
        escapeCsvField(c.phones[0] || ''),
        escapeCsvField(c.notes || ''),
      ].join(','),
    )
  }
  return rows.join('\r\n')
}
