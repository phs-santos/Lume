export type TimeOfDay = "manha" | "tarde" | "noite";

export function timeOfDay(hour: number): TimeOfDay {
  if (hour >= 5 && hour < 12) return "manha";
  if (hour >= 12 && hour < 18) return "tarde";
  return "noite";
}

export const greeting: Record<TimeOfDay, string> = {
  manha: "Bom dia. A página está esperando.",
  tarde: "Uma boa tarde para escrever.",
  noite: "A noite é sua. Escreva.",
};
