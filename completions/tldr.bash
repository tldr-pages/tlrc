# shellcheck shell=bash

_tldr() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"

    local opts="-u -l -a -i -r -p -L -o -c -R -q -v -h \
    --update --list --list-all --info --render --clean-cache \
    --gen-config --config-path --platform --language --offline \
    --compact --no-compact --raw --no-raw --quiet --color --config --version --help"

    if [[ $cur == -* ]]; then
        mapfile -t COMPREPLY < <(compgen -W "$opts" "$cur")
        return 0
    fi

    case $prev in
        -r|--render)
            mapfile -t COMPREPLY < <(compgen -f "$cur")
            return 0;;
        -p|--platform)
            mapfile -t COMPREPLY < <(compgen -f "$cur")
            return 0;;
        -L|--language)
            mapfile -t COMPREPLY < <(compgen -f "$cur")
            return 0;;
        --color)
            mapfile -t COMPREPLY < <(compgen -W "auto always never" "$cur")
            return 0;;
        --config)
            mapfile -t COMPREPLY < <(compgen -f "$cur")
            return 0;;
        *)
            mapfile -t COMPREPLY < <(compgen -W "$(tldr --quiet --offline --list-all)" "$cur");;
    esac
}

complete -o bashdefault -F _tldr tldr
