import { environment } from './fs.js'
import { run } from './proc.js'
import { realNameOf } from './userNames.js'

// Who is logged in, as the lock screen shows them. The login name comes from
// the environment and the display name from the passwd entry's GECOS field.

export interface UserIdentity {
  userName: string
  realName: string
}

export async function currentUser(): Promise<UserIdentity> {
  const userName = loginNameOf(await environment())
  const entry = await run(['getent', 'passwd', userName])
  return { userName, realName: realNameOf(entry.stdout, userName) }
}

function loginNameOf(variables: Record<string, string>): string {
  if (variables.USER !== undefined && variables.USER !== '') {
    return variables.USER
  }
  if (variables.LOGNAME !== undefined) {
    return variables.LOGNAME
  }
  return ''
}
