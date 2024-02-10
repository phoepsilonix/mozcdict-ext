BEGIN{
    FS="\t"
    OFS="\t"
}
{
    if (!a[$1,$5]++) {
        print $0
    }
}
