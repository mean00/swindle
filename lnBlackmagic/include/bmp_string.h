

class stringWrapper
{
public:
        stringWrapper();
        ~stringWrapper();
  void doubleLimit();
  void append(const char *a);
  void appendHexified(const char *a);
  void appendHex32(const uint32_t value);
  char *string() {return _st;}
protected:
  char *_st;
  int _limit;
};
