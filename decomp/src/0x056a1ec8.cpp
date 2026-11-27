/* Process entry (IMAGE_OPTIONAL_HEADER::AddressOfEntryPoint). One call, then a
 * tail jump whose return value is passed straight back to the loader. */
extern "C" void sub_56a1edc(void);
extern "C" int sub_56a1d54(void);

extern "C" int sub_56a1ec8(void)
{
    sub_56a1edc();
    return sub_56a1d54();
}
