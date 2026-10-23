// The runtime has no Intl, so the bar and the lock screen format dates here.

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
const LONG_WEEKDAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
const LONG_MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
]

function twoDigits(value: number): string {
  return String(value).padStart(2, '0')
}

// "Fri, 23 Oct"
export function shortDate(date: Date): string {
  return `${WEEKDAYS[date.getDay()]}, ${date.getDate()} ${MONTHS[date.getMonth()]}`
}

// "Friday, 23 October"
export function longDate(date: Date): string {
  return `${LONG_WEEKDAYS[date.getDay()]}, ${date.getDate()} ${LONG_MONTHS[date.getMonth()]}`
}

// "22:41"
export function clockTime(date: Date): string {
  return `${twoDigits(date.getHours())}:${twoDigits(date.getMinutes())}`
}

export function shortWeekday(date: Date): string {
  return WEEKDAYS[date.getDay()]
}
