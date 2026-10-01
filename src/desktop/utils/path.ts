/** Show the home folder as `~` so paths fit in narrow rows. */
export const shortPath = (path: string) => path.replace(/^\/(?:Users|home)\/[^/]+/, '~');
