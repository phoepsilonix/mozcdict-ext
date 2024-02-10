BEGIN{
    FS="\t"
    OFS="\t"
}
{
    if (!a[$1,$2]++) {
        print $0
    }
}
