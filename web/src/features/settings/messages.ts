// Strings of the settings area: "My account" and "My organisation" navigation and section titles.

import { defineMessages } from "@/i18n/messages";

import type { SectionId } from "./sections";

export const useSettingsMessages = defineMessages({
  en: {
    account: "My account",
    org: "My organisation",
    ns: "Natural Solutions",
    administration: "Administration",
    nav: "Settings",
    title: (id: SectionId) =>
      ({
        profile: "Profile",
        security: "Password",
        gbif: "GBIF account",
        overview: "Overview",
        members: "Members",
        "org-gbif": "GBIF account",
        feedback: "Feedback",
        history: "History",
      })[id],
    text: (id: SectionId) =>
      ({
        profile: "Your name, sign-in e-mail and interface language.",
        security: "Change your password; your other sessions are closed.",
        gbif: "Your own GBIF account, used first for the downloads of your studies.",
        overview: "Your organisation at a glance.",
        members: "Invite colleagues, send password-reset links and disable accounts.",
        "org-gbif": "The GBIF account used by every member who has not set their own.",
        feedback: "What your colleagues sent to Natural Solutions, and whether it has been dealt with.",
        history: "What changed in your organisation and who changed it: invitations, roles, accounts, quota, GBIF account, and when Natural Solutions opened one of your studies for support.",
      })[id],
    managersOnly: "Managers only",
  },
  fr: {
    account: "Mon compte",
    org: "Mon organisation",
    ns: "Natural Solutions",
    administration: "Administration",
    nav: "Paramètres",
    title: (id: SectionId) =>
      ({
        profile: "Profil",
        security: "Mot de passe",
        gbif: "Compte GBIF",
        overview: "Vue d'ensemble",
        members: "Membres",
        "org-gbif": "Compte GBIF",
        feedback: "Avis",
        history: "Historique",
      })[id],
    text: (id: SectionId) =>
      ({
        profile: "Votre nom, votre e-mail de connexion et la langue de l'interface.",
        security: "Changez votre mot de passe ; vos autres sessions sont fermées.",
        gbif: "Votre propre compte GBIF, utilisé en premier pour les téléchargements de vos études.",
        overview: "Votre organisation en un coup d'œil.",
        members: "Invitez des collègues, envoyez des liens de réinitialisation et désactivez des comptes.",
        "org-gbif": "Le compte GBIF utilisé par chaque membre qui n'a pas configuré le sien.",
        feedback: "Ce que vos collègues ont envoyé à Natural Solutions, et s'il a été traité.",
        history: "Ce qui a changé dans votre organisation et qui l'a changé : invitations, rôles, comptes, quota, compte GBIF, et quand Natural Solutions a ouvert l'une de vos études pour le support.",
      })[id],
    managersOnly: "Gestionnaires",
  },
  es: {
    account: "Mi cuenta",
    org: "Mi organización",
    ns: "Natural Solutions",
    administration: "Administración",
    nav: "Ajustes",
    title: (id: SectionId) =>
      ({
        profile: "Perfil",
        security: "Contraseña",
        gbif: "Cuenta de GBIF",
        overview: "Resumen",
        members: "Miembros",
        "org-gbif": "Cuenta de GBIF",
        feedback: "Opiniones",
        history: "Historial",
      })[id],
    text: (id: SectionId) =>
      ({
        profile: "Su nombre, su correo de acceso y el idioma de la interfaz.",
        security: "Cambie su contraseña; sus otras sesiones se cierran.",
        gbif: "Su propia cuenta de GBIF, usada en primer lugar para las descargas de sus estudios.",
        overview: "Su organización de un vistazo.",
        members: "Invite a colegas, envíe enlaces de restablecimiento y desactive cuentas.",
        "org-gbif": "La cuenta de GBIF que usa cada miembro que no ha configurado la suya.",
        feedback: "Lo que sus colegas han enviado a Natural Solutions, y si ya se ha atendido.",
        history: "Qué ha cambiado en su organización y quién lo ha cambiado: invitaciones, roles, cuentas, cuota, cuenta de GBIF, y cuándo Natural Solutions abrió uno de sus estudios para darle soporte.",
      })[id],
    managersOnly: "Gestores",
  },
});
