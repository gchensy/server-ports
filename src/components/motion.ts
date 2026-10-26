/** The one spring the whole UI moves with, so that things feel related. */
export const spring = { type: "spring", stiffness: 420, damping: 34, mass: 0.8 } as const;
export const gentle = { type: "spring", stiffness: 260, damping: 30, mass: 0.9 } as const;
