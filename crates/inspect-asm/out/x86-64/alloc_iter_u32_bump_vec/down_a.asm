inspect_asm::alloc_iter_u32_bump_vec::down_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	movups xmm0, xmmword ptr [rip + .L__unnamed_0]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	test rdx, rdx
	jne .LBB0_3
.LBB0_0:
	mov rax, qword ptr [rsp + 16]
	test rax, rax
	je .LBB0_1
	mov r15, qword ptr [rsp + 24]
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	lea r14, [rsi + 4*rax]
	lea rdx, [4*rbx]
	sub r14, rdx
	mov rdi, r14
	call qword ptr [rip + memmove@GOTPCREL]
	mov rax, r14
	mov rcx, qword ptr [r15]
	mov qword ptr [rcx], r14
	jmp .LBB0_2
.LBB0_1:
	mov eax, 4
	xor ebx, ebx
.LBB0_2:
	mov rdx, rbx
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_3:
	mov rbx, rsp
	mov rdi, rbx
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::mut_bump_vec::MutBumpVec<T,A>::generic_grow_amortized
	mov rcx, r14
	mov rax, r15
	mov rdx, qword ptr [rsp + 8]
	shl rax, 2
	xor r15d, r15d
	jmp .LBB0_5
.LBB0_4:
	mov rsi, qword ptr [rsp]
	mov dword ptr [rsi + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r15, 4
	cmp rax, r15
	je .LBB0_0
.LBB0_5:
	mov ebp, dword ptr [rcx + r15]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_4
	mov esi, 1
	mov rdi, rbx
	mov r12, rax
	call bump_scope::mut_bump_vec::MutBumpVec<T,A>::generic_grow_amortized
	mov rcx, r14
	mov rax, r12
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_4
