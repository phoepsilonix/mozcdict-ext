BEGIN{
    FS="\t"
    OFS="\t"
}
{
    if (!a[$1,$2,$5]++) {
        print $0
    }
}
