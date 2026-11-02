// Messages that accompany a one-time link (NicheRS sends no e-mail: the person creating the
// link pastes this text into their own mail). Written in the recipient's language, which may
// differ from the sender's interface. Scientific wording rules apply: relative suitability,
// never a probability.

import type { Lang } from "@/i18n/lang";

export type LinkKind = "invite" | "reset";

interface Args {
  kind: LinkKind;
  url: string;
  days: number;
  org: string;
  sender: string;
}

const TEXT: Record<Lang, (a: Args) => string> = {
  en: ({ kind, url, days, org, sender }) =>
    kind === "invite"
      ? `Hello,

You are invited to join the NicheRS workspace of “${org}”. NicheRS, by Natural Solutions, maps the relative habitat suitability of species from GBIF occurrence records and climate data, with an evidence level for every map.

To create your account, open this personal link (valid ${days} days, usable once):
${url}

You will choose your password (at least 12 characters). The link is personal: please do not forward it.

${sender}`
      : `Hello,

Here is a link to choose a new NicheRS password (valid ${days} days, usable once):
${url}

If you did not ask for it, ignore this message: your current password stays valid.

${sender}`,
  fr: ({ kind, url, days, org, sender }) =>
    kind === "invite"
      ? `Bonjour,

Vous êtes invité(e) à rejoindre l'espace NicheRS de « ${org} ». NicheRS, de Natural Solutions, cartographie l'idonéité relative des habitats des espèces à partir des occurrences GBIF et des données climatiques, avec un niveau de preuve pour chaque carte.

Pour créer votre compte, ouvrez ce lien personnel (valable ${days} jours, utilisable une seule fois) :
${url}

Vous choisirez votre mot de passe (12 caractères au moins). Ce lien est personnel : merci de ne pas le transférer.

${sender}`
      : `Bonjour,

Voici un lien pour choisir un nouveau mot de passe NicheRS (valable ${days} jours, utilisable une seule fois) :
${url}

Si vous n'avez rien demandé, ignorez ce message : votre mot de passe actuel reste valable.

${sender}`,
  es: ({ kind, url, days, org, sender }) =>
    kind === "invite"
      ? `Hola:

Le invitamos a unirse al espacio de NicheRS de «${org}». NicheRS, de Natural Solutions, cartografía la idoneidad relativa del hábitat de las especies a partir de registros de GBIF y datos climáticos, con un nivel de evidencia para cada mapa.

Para crear su cuenta, abra este enlace personal (válido ${days} días, de un solo uso):
${url}

Elegirá su contraseña (12 caracteres como mínimo). El enlace es personal: no lo reenvíe, por favor.

${sender}`
      : `Hola:

Este es un enlace para elegir una nueva contraseña de NicheRS (válido ${days} días, de un solo uso):
${url}

Si no lo ha pedido, ignore este mensaje: su contraseña actual sigue siendo válida.

${sender}`,
};

export function linkMessage(lang: Lang, a: Args): string {
  return TEXT[lang](a);
}
