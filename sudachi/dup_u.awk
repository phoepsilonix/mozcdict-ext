BEGIN{
    FS="\t"
    OFS="\t"
}
{
    if (!a[$1,$2,$3]++) {
        print $0
    }
}
