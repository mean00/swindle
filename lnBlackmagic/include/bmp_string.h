

class stringWrapper
{
public:
        stringWrapper();
        ~stringWrapper();
  void doubleLimit();
  void append(const char *a);
  void appendHex32(const uint32_t value);
  void appendHexified(const char *a);
  void appendU32(uint32_t val);
  char *string() {return _st;}
protected:
  char *_st;
  int _limit;
};
