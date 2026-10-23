// The extensions that talk to a web API (open-meteo, the air quality endpoint)
// need nothing more than "GET this URL, give me the JSON".

export async function getJson<Result>(url: string): Promise<Result> {
  return JSON.parse(await getText(url)) as Result
}

export async function getText(url: string): Promise<string> {
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`http: ${url} returned ${response.status}`)
  }
  return response.text()
}

export function withQuery(url: string, params: Record<string, string | number>): string {
  const pairs: string[] = []
  for (const [key, value] of Object.entries(params)) {
    pairs.push(`${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`)
  }
  if (pairs.length === 0) {
    return url
  }
  return `${url}?${pairs.join('&')}`
}
