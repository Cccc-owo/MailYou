const SIGNATURE_START = '<!--mailyou-signature:start:'
const SIGNATURE_END = '<!--mailyou-signature:end-->'

const escapeRegExp = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

export const applyIdentitySignature = (
  body: string,
  identityId: string,
  signature?: string | null,
  force = false,
) => {
  const cleanedBody = removeIdentitySignature(body)
  const nextSignature = signature?.trim()

  if (!nextSignature) {
    return cleanedBody
  }

  if (!force && cleanedBody.trim().length > 0) {
    return cleanedBody
  }

  const block = `${SIGNATURE_START}${identityId}-->${nextSignature}${SIGNATURE_END}`
  return `${cleanedBody}<br><br>${block}`.trim()
}

export const removeIdentitySignature = (body: string) => {
  const pattern = new RegExp(`${escapeRegExp(SIGNATURE_START)}.*?-->[\\s\\S]*?${escapeRegExp(SIGNATURE_END)}`, 'g')
  return body.replace(pattern, '').replace(/(<br>\s*){3,}$/g, '<br><br>').trim()
}
