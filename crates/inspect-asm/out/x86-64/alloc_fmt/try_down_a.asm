inspect_asm::alloc_fmt::try_down_a:
	push r15
	push r14
	push rbx
	sub rsp, 112
	mov qword ptr [rsp + 32], rsi
	mov qword ptr [rsp + 40], rdx
	lea rax, [rsp + 32]
	mov qword ptr [rsp + 48], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 64], rax
	mov qword ptr [rsp + 72], 2
	mov qword ptr [rsp + 96], 0
	lea rax, [rsp + 48]
	mov qword ptr [rsp + 80], rax
	mov qword ptr [rsp + 88], 1
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 64]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB0_1
	mov rcx, qword ptr [rsp]
	mov rax, qword ptr [rsp + 24]
	mov rax, qword ptr [rax]
	cmp qword ptr [rax], rcx
	jne .LBB0_0
	mov rdx, qword ptr [rsp + 16]
	add rcx, rdx
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
.LBB0_0:
	xor eax, eax
	jmp .LBB0_5
.LBB0_1:
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 24]
	mov rax, qword ptr [r14]
	cmp rsi, qword ptr [rax]
	je .LBB0_2
	mov rax, rsi
	jmp .LBB0_5
.LBB0_2:
	mov rax, qword ptr [rsp + 16]
	add rax, rsi
	xor edi, edi
	sub rax, rbx
	cmovae rdi, rax
	and rdi, -4
	lea rax, [rbx + rsi]
	mov r15, rdi
	mov rdx, rbx
	cmp rax, rdi
	jbe .LBB0_3
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_4
.LBB0_3:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_4:
	mov rcx, qword ptr [r14]
	mov rax, r15
	mov qword ptr [rcx], r15
.LBB0_5:
	mov rdx, rbx
	add rsp, 112
	pop rbx
	pop r14
	pop r15
	ret
	mov rdx, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 24]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rdx
	jne .LBB0_6
	mov rsi, qword ptr [rsp + 16]
	add rdx, rsi
	add rdx, 3
	and rdx, -4
	mov qword ptr [rcx], rdx
.LBB0_6:
	mov rdi, rax
	call _Unwind_Resume@PLT
