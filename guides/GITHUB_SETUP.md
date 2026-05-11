# Initialiser un repo GitHub et envoyer son code

## Prérequis

- [Git](https://git-scm.com/) installé
- [GitHub CLI (`gh`)](https://cli.github.com/) installé
- Être authentifié : `gh auth login`

---

## 1. Initialiser le repo local

Dans le dossier de ton projet :

```bash
git init
git add .
git commit -m "Initial commit"
```

> `git add .` ajoute tous les fichiers non exclus par `.gitignore`.

---

## 2. Créer le repo GitHub et pusher en une commande

```bash
gh repo create <utilisateur>/<nom-du-repo> --public --source=. --remote=origin --push
```

**Options utiles :**

| Option | Description |
|---|---|
| `--public` | Repo public (remplacer par `--private` pour privé) |
| `--source=.` | Utilise le dossier courant comme source |
| `--remote=origin` | Nomme le remote `origin` |
| `--push` | Pousse le code immédiatement |

---

## 3. Pusher les modifications suivantes

Une fois le repo créé, les commits suivants se poussent avec :

```bash
git add .
git commit -m "Description des changements"
git push
```

---

## Alternative : lier un repo GitHub existant

Si le repo existe déjà sur GitHub :

```bash
git remote add origin https://github.com/<utilisateur>/<nom-du-repo>.git
git branch -M main
git push -u origin main
```

---

## Vérifier la configuration du remote

```bash
git remote -v
```
