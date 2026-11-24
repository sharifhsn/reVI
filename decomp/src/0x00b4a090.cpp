/* Behaviourally equal to the original; see the codegen note. */
struct O {
    char pad[0x108];
    double v;
};

float sub_b4a090(struct O *o, unsigned i)
{
    double d = o->v;
    return ((float *)&d)[i];
}
