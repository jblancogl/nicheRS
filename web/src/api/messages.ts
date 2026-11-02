// Client-side error texts of the API wrapper (server-provided problem titles stay as they come).

import { defineMessages } from "@/i18n/messages";

export const apiMessages = defineMessages({
  en: {
    unreachable: "The NicheRS server could not be reached. Check that it is running, then retry.",
    notResponding: "The NicheRS server is not responding. Check that it is running, then retry.",
    httpFailed: (status: number) => `Request failed (HTTP ${status}).`,
    notJson: "Unexpected response from the server (not JSON). Is the API running behind this address?",
    unknown: "Unknown error.",
  },
  fr: {
    unreachable: "Impossible de joindre le serveur NicheRS. Vérifiez qu'il est démarré, puis réessayez.",
    notResponding: "Le serveur NicheRS ne répond pas. Vérifiez qu'il est démarré, puis réessayez.",
    httpFailed: (status: number) => `La requête a échoué (HTTP ${status}).`,
    notJson: "Réponse inattendue du serveur (pas du JSON). L'API fonctionne-t-elle derrière cette adresse ?",
    unknown: "Erreur inconnue.",
  },
  es: {
    unreachable: "No se ha podido contactar con el servidor NicheRS. Compruebe que está en marcha y vuelva a intentarlo.",
    notResponding: "El servidor NicheRS no responde. Compruebe que está en marcha y vuelva a intentarlo.",
    httpFailed: (status: number) => `La solicitud ha fallado (HTTP ${status}).`,
    notJson: "Respuesta inesperada del servidor (no es JSON). ¿Funciona la API detrás de esta dirección?",
    unknown: "Error desconocido.",
  },
});
